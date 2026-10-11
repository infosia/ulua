//! S1 subset checks. Later slices extend the typed forms here.
use crate::{Diagnostic, ir};
use oxc_ast::ast::*;
use oxc_span::GetSpan;

pub(crate) fn check(program: &Program<'_>, source: &[u8]) -> Result<ir::Module, Diagnostic> {
    let unsupported = |offset| {
        Diagnostic::at(
            source,
            offset,
            "unsupported",
            "This form is outside the implemented subset",
        )
    };
    if program.hashbang.is_some() || !program.directives.is_empty() || program.body.len() != 1 {
        return Err(unsupported(
            program.body.first().map_or(0, |s| s.span().start as usize),
        ));
    }
    let Statement::ExportDeclaration(export) = &program.body[0] else {
        return Err(unsupported(program.body[0].span().start as usize));
    };
    let Declaration::FunctionDeclaration(function) = &export.declaration else {
        return Err(unsupported(export.span.start as usize));
    };
    let offset = function.span.start as usize;
    if function.r#async
        || function.generator
        || function.declare
        || function.type_parameters.is_some()
        || function.this_param.is_some()
        || !function.params.items.is_empty()
        || function.params.rest.is_some()
        || function
            .id
            .as_ref()
            .is_none_or(|id| id.name.as_str() != "main")
        || function
            .return_type
            .as_ref()
            .is_none_or(|t| !matches!(t.type_annotation, TSType::TSVoidKeyword(_)))
    {
        return Err(unsupported(offset));
    }
    let body = function.body.as_ref().ok_or_else(|| unsupported(offset))?;
    if !body.directives.is_empty() || body.statements.len() != 1 {
        return Err(unsupported(body.span.start as usize));
    }
    let Statement::ExpressionStatement(statement) = &body.statements[0] else {
        return Err(unsupported(body.statements[0].span().start as usize));
    };
    let Expression::CallExpression(call) = &statement.expression else {
        return Err(unsupported(statement.span.start as usize));
    };
    if call.optional
        || call.type_arguments.is_some()
        || call.arguments.len() != 1
        || !matches!(&call.callee, Expression::Identifier(id) if id.name.as_str() == "print")
    {
        return Err(unsupported(call.span.start as usize));
    }
    let Argument::StringLiteral(literal) = &call.arguments[0] else {
        return Err(unsupported(call.arguments[0].span().start as usize));
    };
    if literal.lone_surrogates {
        return Err(Diagnostic::at(
            source,
            literal.span.start as usize,
            "C5",
            "String contains an unpaired surrogate",
        ));
    }
    Ok(ir::Module {
        functions: vec![ir::Function {
            name: "main".into(),
            body: vec![ir::Statement::Print(literal.value.as_str().to_owned())],
        }],
    })
}
