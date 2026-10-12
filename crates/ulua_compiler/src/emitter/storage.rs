//! Storage plans assign captures to their declaring block.
use super::*;
use functions::Layout;

type SpillField = (String, String, &'static str);
type SpillTables = std::collections::BTreeMap<(usize, usize), Vec<SpillField>>;

impl Emitter<'_> {
    pub(super) fn plan(
        &mut self,
        parameters: &[usize],
        overhead: usize,
        groups: &[usize],
    ) -> Result<Layout, Diagnostic> {
        self.offset = self.storage_offset;
        let fixed = parameters.len() + self.slots.len() + self.generated.len() + overhead;
        if self.slots.len() + self.generated.len() + overhead > TEMPORARY_BOUND {
            return Err(self.limit("limit:temporaries", "Non-spill local limit exceeded"));
        }
        let ids: Vec<_> = self
            .binding_ids
            .iter()
            .copied()
            .filter(|id| !parameters.contains(id))
            .collect();
        let capacity = LOCAL_BUDGET
            .checked_sub(fixed)
            .ok_or_else(|| self.limit("limit:temporaries", "No room for binding storage"))?;
        let mut table_count = usize::from(ids.len() > capacity);
        let mut fields_per_table = SPILL_FIELDS;
        let group = |id: usize| {
            if self.module.captured[id] {
                groups[id]
            } else {
                0
            }
        };
        loop {
            let available = capacity.checked_sub(table_count).ok_or_else(|| {
                self.limit(
                    "limit:temporaries",
                    "Block spill locals exceed the local budget",
                )
            })?;
            let mut counts = HashMap::<usize, usize>::new();
            for &id in ids.iter().skip(available) {
                *counts.entry(group(id)).or_default() += 1;
            }
            if counts.len() > capacity {
                let mut seen = std::collections::HashSet::new();
                if let Some(id) = ids
                    .iter()
                    .skip(available)
                    .find(|id| seen.insert(group(**id)) && seen.len() > capacity)
                {
                    self.offset = self.module.bindings[*id].start as usize;
                }
                return Err(self.limit(
                    "limit:temporaries",
                    "Block spill locals exceed the local budget",
                ));
            }
            let mut needed: usize = counts.values().map(|n| n.div_ceil(fields_per_table)).sum();
            if needed > capacity {
                let mut low = fields_per_table;
                let mut high = counts.values().copied().max().unwrap_or(fields_per_table);
                while low < high {
                    let middle = low + (high - low) / 2;
                    let count: usize = counts.values().map(|n| n.div_ceil(middle)).sum();
                    if count <= capacity {
                        high = middle;
                    } else {
                        low = middle + 1;
                    }
                }
                fields_per_table = low;
                needed = counts.values().map(|n| n.div_ceil(fields_per_table)).sum();
            }
            if needed == table_count {
                break;
            }
            table_count = needed;
        }
        let available = capacity - table_count;
        let mut layout = Layout {
            headers: String::new(),
            entries: HashMap::new(),
            slots: self.slots.clone(),
        };
        let mut tables = SpillTables::new();
        let mut field_ids = HashMap::new();
        let mut counts = HashMap::<usize, usize>::new();
        for (i, id) in ids.iter().copied().enumerate() {
            let ty = self.module.bindings[id]
                .ty
                .as_ref()
                .ok_or_else(|| self.limit("internal", "Missing storage type"))?;
            let name = format!("__ulua_b{id}");
            let key = group(id);
            if i < available {
                let header = if key == 0 {
                    &mut layout.headers
                } else {
                    layout.entries.entry(key).or_default()
                };
                let _ = writeln!(
                    header,
                    "local {name}: {0} = ({1}) :: {0}",
                    self.function_types.luau(ty),
                    initial(ty)
                );
                self.locations.insert(id, name);
            } else {
                field_ids.insert(name.clone(), id);
                let index = counts.entry(key).or_default();
                tables
                    .entry((key, *index / fields_per_table))
                    .or_default()
                    .push((name, self.function_types.luau(ty), initial(ty)));
                *index += 1;
            }
        }
        for (table, ((key, _), fields)) in tables.into_iter().enumerate() {
            let name = if table == 0 {
                format!("__ulua_spill{}", self.owner)
            } else {
                format!("__ulua_spill{}_{table}", self.owner)
            };
            for (index, (field, _, _)) in fields.iter().enumerate() {
                let page = if fields.len() > SPILL_FIELDS {
                    format!(".__ulua_page{}", index / SPILL_FIELDS)
                } else {
                    String::new()
                };
                if let Some(&id) = field_ids.get(field) {
                    self.locations.insert(id, format!("{name}{page}.{field}"));
                }
            }
            let (ty, value) = types::spill_constructor(&fields);
            let header = if key == 0 {
                &mut layout.headers
            } else {
                layout.entries.entry(key).or_default()
            };
            let _ = writeln!(header, "local {name}: {ty} = ({value}) :: {ty}");
        }
        for (i, slot) in self.slots.iter().enumerate() {
            let _ = writeln!(
                layout.headers,
                "local {0}: {1} = ({2}) :: {1}",
                self.temp(i),
                self.function_types.luau(&slot.ty),
                initial(&slot.ty)
            );
        }
        for (name, ty, value, entry) in &self.generated {
            if *entry {
                let _ = writeln!(layout.headers, "local {name}: {ty} = ({value}) :: {ty}");
            }
        }
        Ok(layout)
    }
}
