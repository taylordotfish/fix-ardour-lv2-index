/*
 * Copyright (C) 2024 taylor.fish <contact@taylor.fish>
 *
 * This file is part of fix-ardour-lv2-index.
 *
 * fix-ardour-lv2-index is free software: you can redistribute it and/or
 * modify it under the terms of the GNU General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * fix-ardour-lv2-index is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
 * General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along
 * with fix-ardour-lv2-index. If not, see <https://www.gnu.org/licenses/>.
 */

use roxmltree::Node;
use std::collections::HashMap;
use std::ops::Range;
use std::str::FromStr;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct ParameterIndex(u32);

impl FromStr for ParameterIndex {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(());
        }
        s.parse().map(Self).map_err(|_| ())
    }
}

#[derive(Debug)]
pub struct Parameter<'a> {
    pub symbol: &'a str,
    pub location: Range<usize>,
    pub old_index: u32,
}

#[derive(Debug)]
pub struct Processor<'a> {
    uri: &'a str,
    symbols: HashMap<ParameterIndex, &'a str>,
    parameters: Vec<(ParameterIndex, Range<usize>)>,
}

impl<'a> Processor<'a> {
    pub fn uri(&self) -> &'a str {
        self.uri
    }

    pub fn parameters(&self) -> impl Iterator<Item = Parameter<'a>> + '_ {
        self.parameters.iter().filter_map(|(i, range)| {
            self.symbols.get(i).map(|&s| Parameter {
                symbol: s,
                location: range.clone(),
                old_index: i.0,
            })
        })
    }

    fn on_automation_list(&mut self, node: Node<'a, '_>) {
        let Some(attr) = node.attribute_node("automation-id") else {
            return;
        };
        const PREFIX: &str = "parameter-";
        let Some(index) = attr.value().strip_prefix(PREFIX) else {
            return;
        };
        let Ok(parsed_index) = index.parse() else {
            eprintln!("warning: could not parse parameter index: {index}");
            return;
        };
        let mut range = attr.range_value();
        range.start += PREFIX.len();
        self.parameters.push((parsed_index, range));
    }

    fn on_controllable(&mut self, node: Node<'a, '_>) {
        let Some(index_attr) = node.attribute_node("parameter") else {
            return;
        };
        let index = index_attr.value();
        let Ok(parsed_index) = index.parse() else {
            eprintln!("warning: could not parse parameter index: {index}");
            return;
        };
        let Some(symbol) = node.attribute("symbol") else {
            eprintln!(
                "warning: missing `symbol` in controllable at {}",
                node.range().start,
            );
            return;
        };
        self.symbols.insert(parsed_index, symbol);
        self.parameters.push((parsed_index, index_attr.range_value()));
    }

    pub fn parse(node: Node<'a, '_>) -> Option<Self> {
        if node.attribute("type") != Some("lv2") {
            return None;
        }
        let Some(uri) = node.attribute("unique-id") else {
            eprintln!(
                "warning: missing uri for processor at {}",
                node.range().start,
            );
            return None;
        };
        let mut this = Self {
            uri,
            symbols: HashMap::new(),
            parameters: Vec::new(),
        };
        let mut next = node.first_child();
        while let Some(descendant) = next {
            next = None;
            if descendant.has_tag_name("AutomationList") {
                this.on_automation_list(descendant);
            } else if descendant.has_tag_name("Controllable") {
                this.on_controllable(descendant);
            } else {
                next = descendant.first_child();
            }
            next = next.or_else(|| {
                descendant
                    .ancestors()
                    .take_while(|a| a != &node)
                    .filter_map(|a| a.next_sibling())
                    .next()
            });
        }
        Some(this)
    }
}
