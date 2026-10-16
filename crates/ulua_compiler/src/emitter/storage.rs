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
        capture_reserve: usize,
    ) -> Result<Layout, Diagnostic> {
        self.offset = self.storage_offset;
        let fixed = parameters.len() + self.slots.len() + self.generated_peak + overhead;
        if self.slots.len() + self.generated_peak + overhead > TEMPORARY_BOUND {
            return Err(self.limit("limit:temporaries", "Non-spill local limit exceeded"));
        }
        let mut captured = 0;
        let mut forced = std::collections::HashSet::new();
        for &id in parameters.iter().chain(
            self.binding_ids
                .iter()
                .filter(|id| !parameters.contains(id)),
        ) {
            if self.module.captured[id] {
                captured += 1;
                if captured > crate::CAPTURE_LOCALS - capture_reserve {
                    forced.insert(id);
                }
            }
        }
        let mut ids: Vec<_> = self
            .binding_ids
            .iter()
            .copied()
            .filter(|id| !parameters.contains(id) || forced.contains(id))
            .collect();
        // Preserve source order within each class. Forced spills cannot consume a local slot.
        ids.sort_by_key(|id| forced.contains(id));
        let eligible = ids.iter().take_while(|id| !forced.contains(id)).count();
        let capacity = LOCAL_BUDGET
            .checked_sub(fixed)
            .ok_or_else(|| self.limit("limit:temporaries", "No room for binding storage"))?;
        let mut table_count = usize::from(ids.len() > capacity || eligible < ids.len());
        let mut fields_per_table = SPILL_FIELDS;
        let group = |id: usize| {
            if self.module.captured[id] && !parameters.contains(&id) {
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
            for &id in ids.iter().skip(available.min(eligible)) {
                *counts.entry(group(id)).or_default() += 1;
            }
            if counts.len() > capacity {
                let mut seen = std::collections::HashSet::new();
                if let Some(id) = ids
                    .iter()
                    .skip(available.min(eligible))
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
        let available = (capacity - table_count).min(eligible);
        let mut layout = Layout {
            overhead,
            headers: String::new(),
            entries: HashMap::new(),
            entry_locals: HashMap::new(),
            slots: self.slots.clone(),
            protected_slots: std::mem::take(&mut self.protected_slots),
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
                layout
                    .entry_locals
                    .entry(key)
                    .or_default()
                    .insert(name.clone());
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
                self.spilled.insert(id);
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
            layout
                .entry_locals
                .entry(key)
                .or_default()
                .insert(name.clone());
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
            let (ty, _) = types::spill_constructor(&fields);
            let header = if key == 0 {
                &mut layout.headers
            } else {
                layout.entries.entry(key).or_default()
            };
            // Each statement initializes one field. Storage pages retain their fixed layout.
            let _ = writeln!(header, "local {name}: {ty} = ({{}} :: any) :: {ty}");
            if fields.len() <= SPILL_FIELDS {
                initialize_fields(header, &name, &fields);
            } else {
                for (index, fields) in fields.chunks(SPILL_FIELDS).enumerate() {
                    let (page_type, _) = types::spill_constructor(fields);
                    let page = format!("{name}.__ulua_page{index}");
                    let _ = writeln!(header, "{page} = ({{}} :: any) :: {page_type}");
                    initialize_fields(header, &page, fields);
                }
            }
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
        Ok(layout)
    }
}

fn initialize_fields(header: &mut String, receiver: &str, fields: &[(String, String, &str)]) {
    for (field, ty, value) in fields {
        let _ = writeln!(header, "{receiver}.{field} = ({value}) :: {ty}");
    }
}
