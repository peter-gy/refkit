use hayagriva::citationberg::taxonomy::{NumberVariable, Variable};
use hayagriva::citationberg::{
    IndependentStyle, InheritableNameOptions, LayoutRenderingElement, Names, Sort, SortKey,
    TextTarget,
};

pub(crate) fn citation_only_style(style: &IndependentStyle) -> IndependentStyle {
    let mut style = style.clone();
    style.bibliography = None;
    style
}

pub(crate) fn full_history_citation_style(style: &IndependentStyle) -> Option<IndependentStyle> {
    if !citation_depends_on_citation_number(style) || bibliography_has_sort(style) {
        return None;
    }

    Some(citation_only_style(style))
}

pub(crate) fn can_fast_render_single_citations(style: &IndependentStyle) -> bool {
    !citation_depends_on_citation_number(style)
        && !citation_depends_on_position(style)
        && !citation_depends_on_subsequent_names(style)
}

pub(crate) fn citation_depends_on_subsequent_names(style: &IndependentStyle) -> bool {
    elements_depend_on_subsequent_names(
        &style.citation.layout.elements,
        &style.citation.name_options,
        style,
        &mut Vec::new(),
    )
}

fn bibliography_has_sort(style: &IndependentStyle) -> bool {
    style
        .bibliography
        .as_ref()
        .and_then(|bibliography| bibliography.sort.as_ref())
        .is_some()
}

fn citation_depends_on_citation_number(style: &IndependentStyle) -> bool {
    let citation_number = Variable::Number(NumberVariable::CitationNumber);
    elements_depend_on_variable(
        &style.citation.layout.elements,
        citation_number,
        style,
        &mut Vec::new(),
    ) || sort_uses_variable(style.citation.sort.as_ref(), citation_number, style)
}

fn sort_uses_variable(sort: Option<&Sort>, variable: Variable, style: &IndependentStyle) -> bool {
    let Some(sort) = sort else {
        return false;
    };
    sort.keys.iter().any(|key| match key {
        SortKey::Variable {
            variable: sort_variable,
            ..
        } => *sort_variable == variable,
        SortKey::MacroName { name, .. } => style
            .macros
            .iter()
            .find(|csl_macro| csl_macro.name == *name)
            .is_some_and(|csl_macro| {
                elements_depend_on_variable(&csl_macro.children, variable, style, &mut Vec::new())
            }),
    })
}

fn elements_depend_on_variable(
    elements: &[LayoutRenderingElement],
    variable: Variable,
    style: &IndependentStyle,
    seen_macros: &mut Vec<String>,
) -> bool {
    elements
        .iter()
        .any(|element| element_depends_on_variable(element, variable, style, seen_macros))
}

fn element_depends_on_variable(
    element: &LayoutRenderingElement,
    variable: Variable,
    style: &IndependentStyle,
    seen_macros: &mut Vec<String>,
) -> bool {
    match element {
        LayoutRenderingElement::Text(text) => match &text.target {
            TextTarget::Variable { var, .. } => *var == variable,
            TextTarget::Macro { name } => {
                macro_depends_on_variable(name, variable, style, seen_macros)
            }
            TextTarget::Term { .. } | TextTarget::Value { .. } => false,
        },
        LayoutRenderingElement::Date(date) => date.variable.map(Variable::Date) == Some(variable),
        LayoutRenderingElement::Number(number) => Variable::from(number.variable) == variable,
        LayoutRenderingElement::Names(names) => {
            if names
                .variable
                .iter()
                .any(|name_variable| Variable::Name(*name_variable) == variable)
            {
                return true;
            }
            names.substitute().is_some_and(|substitute| {
                elements_depend_on_variable(&substitute.children, variable, style, seen_macros)
            })
        }
        LayoutRenderingElement::Label(label) => Variable::from(label.variable) == variable,
        LayoutRenderingElement::Group(group) => {
            elements_depend_on_variable(&group.children, variable, style, seen_macros)
        }
        LayoutRenderingElement::Choose(choose) => {
            choose.branches().any(|branch| {
                elements_depend_on_variable(&branch.children, variable, style, seen_macros)
            }) || choose.otherwise.as_ref().is_some_and(|branch| {
                elements_depend_on_variable(&branch.children, variable, style, seen_macros)
            })
        }
    }
}

fn macro_depends_on_variable(
    name: &str,
    variable: Variable,
    style: &IndependentStyle,
    seen_macros: &mut Vec<String>,
) -> bool {
    if seen_macros.iter().any(|seen| seen == name) {
        return false;
    }

    let Some(csl_macro) = style.macros.iter().find(|csl_macro| csl_macro.name == name) else {
        return false;
    };

    seen_macros.push(name.to_string());
    let depends = elements_depend_on_variable(&csl_macro.children, variable, style, seen_macros);
    seen_macros.pop();
    depends
}

fn citation_depends_on_position(style: &IndependentStyle) -> bool {
    elements_depend_on_position(&style.citation.layout.elements, style, &mut Vec::new())
}

fn elements_depend_on_position(
    elements: &[LayoutRenderingElement],
    style: &IndependentStyle,
    seen_macros: &mut Vec<String>,
) -> bool {
    elements
        .iter()
        .any(|element| element_depends_on_position(element, style, seen_macros))
}

fn element_depends_on_position(
    element: &LayoutRenderingElement,
    style: &IndependentStyle,
    seen_macros: &mut Vec<String>,
) -> bool {
    match element {
        LayoutRenderingElement::Text(text) => match &text.target {
            TextTarget::Macro { name } => macro_depends_on_position(name, style, seen_macros),
            TextTarget::Variable { .. } | TextTarget::Term { .. } | TextTarget::Value { .. } => {
                false
            }
        },
        LayoutRenderingElement::Group(group) => {
            elements_depend_on_position(&group.children, style, seen_macros)
        }
        LayoutRenderingElement::Names(names) => names.substitute().is_some_and(|substitute| {
            elements_depend_on_position(&substitute.children, style, seen_macros)
        }),
        LayoutRenderingElement::Choose(choose) => {
            choose.branches().any(|branch| {
                branch.position.is_some()
                    || elements_depend_on_position(&branch.children, style, seen_macros)
            }) || choose.otherwise.as_ref().is_some_and(|branch| {
                elements_depend_on_position(&branch.children, style, seen_macros)
            })
        }
        LayoutRenderingElement::Date(_)
        | LayoutRenderingElement::Number(_)
        | LayoutRenderingElement::Label(_) => false,
    }
}

fn macro_depends_on_position(
    name: &str,
    style: &IndependentStyle,
    seen_macros: &mut Vec<String>,
) -> bool {
    if seen_macros.iter().any(|seen| seen == name) {
        return false;
    }

    let Some(csl_macro) = style.macros.iter().find(|csl_macro| csl_macro.name == name) else {
        return false;
    };

    seen_macros.push(name.to_string());
    let depends = elements_depend_on_position(&csl_macro.children, style, seen_macros);
    seen_macros.pop();
    depends
}

fn elements_depend_on_subsequent_names(
    elements: &[LayoutRenderingElement],
    inherited: &InheritableNameOptions,
    style: &IndependentStyle,
    seen_macros: &mut Vec<String>,
) -> bool {
    elements
        .iter()
        .any(|element| element_depends_on_subsequent_names(element, inherited, style, seen_macros))
}

fn element_depends_on_subsequent_names(
    element: &LayoutRenderingElement,
    inherited: &InheritableNameOptions,
    style: &IndependentStyle,
    seen_macros: &mut Vec<String>,
) -> bool {
    match element {
        LayoutRenderingElement::Text(text) => match &text.target {
            TextTarget::Macro { name } => {
                macro_depends_on_subsequent_names(name, inherited, style, seen_macros)
            }
            TextTarget::Variable { .. } | TextTarget::Term { .. } | TextTarget::Value { .. } => {
                false
            }
        },
        LayoutRenderingElement::Names(names) => {
            names_depend_on_subsequent_names(names, inherited, style, seen_macros)
        }
        LayoutRenderingElement::Group(group) => {
            elements_depend_on_subsequent_names(&group.children, inherited, style, seen_macros)
        }
        LayoutRenderingElement::Choose(choose) => {
            choose.branches().any(|branch| {
                elements_depend_on_subsequent_names(&branch.children, inherited, style, seen_macros)
            }) || choose.otherwise.as_ref().is_some_and(|branch| {
                elements_depend_on_subsequent_names(&branch.children, inherited, style, seen_macros)
            })
        }
        LayoutRenderingElement::Date(_)
        | LayoutRenderingElement::Number(_)
        | LayoutRenderingElement::Label(_) => false,
    }
}

fn names_depend_on_subsequent_names(
    names: &Names,
    inherited: &InheritableNameOptions,
    style: &IndependentStyle,
    seen_macros: &mut Vec<String>,
) -> bool {
    let inherited = inherited.apply(&names.options());
    if inherited.et_al_subsequent_min.is_some() || inherited.et_al_subsequent_use_first.is_some() {
        return true;
    }

    if let Some(name) = names.name() {
        let options = name.options(&inherited);
        if options.et_al_subsequent_min.is_some() || options.et_al_subsequent_use_first.is_some() {
            return true;
        }
    }

    names.substitute().is_some_and(|substitute| {
        elements_depend_on_subsequent_names(&substitute.children, &inherited, style, seen_macros)
    })
}

fn macro_depends_on_subsequent_names(
    name: &str,
    inherited: &InheritableNameOptions,
    style: &IndependentStyle,
    seen_macros: &mut Vec<String>,
) -> bool {
    if seen_macros.iter().any(|seen| seen == name) {
        return false;
    }

    let Some(csl_macro) = style.macros.iter().find(|csl_macro| csl_macro.name == name) else {
        return false;
    };

    seen_macros.push(name.to_string());
    let depends =
        elements_depend_on_subsequent_names(&csl_macro.children, inherited, style, seen_macros);
    seen_macros.pop();
    depends
}

#[cfg(test)]
mod tests {
    use hayagriva::archive;
    use hayagriva::citationberg::Style as CslStyle;

    use super::*;

    #[test]
    fn citation_only_style_keeps_bibliography_sorted_numbers_on_full_path() {
        let apa = archived_independent_style("apa");
        let ieee = archived_independent_style("ieee");
        let sorted_numbers = independent_style_from_xml(
            r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info>
    <title>Sorted Numbers</title>
    <id>https://example.com/sorted-numbers</id>
    <updated>2024-01-01T00:00:00+00:00</updated>
  </info>
  <citation>
    <layout>
      <number variable="citation-number"/>
    </layout>
  </citation>
  <bibliography>
    <sort>
      <key variable="title"/>
    </sort>
    <layout>
      <text variable="title"/>
    </layout>
  </bibliography>
</style>"#,
        );

        assert!(full_history_citation_style(&apa).is_none());
        assert!(full_history_citation_style(&ieee).is_some());
        assert!(full_history_citation_style(&sorted_numbers).is_none());
        assert!(!can_fast_render_single_citations(&ieee));
    }

    fn archived_independent_style(name: &str) -> IndependentStyle {
        let style = archive::ArchivedStyle::by_name(name)
            .unwrap_or_else(|| panic!("missing archived style {name}"))
            .get();
        match style {
            CslStyle::Independent(style) => style,
            CslStyle::Dependent(_) => panic!("expected independent style {name}"),
        }
    }

    fn independent_style_from_xml(xml: &str) -> IndependentStyle {
        match CslStyle::from_xml(xml).unwrap() {
            CslStyle::Independent(style) => style,
            CslStyle::Dependent(_) => panic!("expected independent style"),
        }
    }
}
