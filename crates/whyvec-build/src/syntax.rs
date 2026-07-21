use std::collections::BTreeMap;

use proc_macro2::LineColumn;
use sha2::{Digest, Sha256};
use syn::spanned::Spanned as _;
use syn::visit::Visit as _;

use crate::git::{SyntaxEditGroup, SyntaxEditGroupSummary, TextHunk};

#[derive(Clone, Debug)]
struct Region {
    start: usize,
    end: usize,
    kind: &'static str,
    symbol: Option<String>,
}

#[derive(Default)]
struct RegionCollector {
    regions: Vec<Region>,
}

macro_rules! collect_named_region {
    ($method:ident, $node:ty, $kind:literal, $walk:path) => {
        fn $method(&mut self, node: &'ast $node) {
            self.push(
                node.span().start(),
                node.span().end(),
                $kind,
                Some(node.sig.ident.to_string()),
            );
            $walk(self, node);
        }
    };
}

impl RegionCollector {
    fn push(
        &mut self,
        start: LineColumn,
        end: LineColumn,
        kind: &'static str,
        symbol: Option<String>,
    ) {
        if start.line > 0 && end.line >= start.line {
            self.regions.push(Region {
                start: start.line,
                end: end.line,
                kind,
                symbol,
            });
        }
    }
}

impl<'ast> syn::visit::Visit<'ast> for RegionCollector {
    collect_named_region!(
        visit_item_fn,
        syn::ItemFn,
        "function",
        syn::visit::visit_item_fn
    );
    collect_named_region!(
        visit_impl_item_fn,
        syn::ImplItemFn,
        "method",
        syn::visit::visit_impl_item_fn
    );
    collect_named_region!(
        visit_trait_item_fn,
        syn::TraitItemFn,
        "trait_method",
        syn::visit::visit_trait_item_fn
    );

    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.push(
            node.span().start(),
            node.span().end(),
            "struct",
            Some(node.ident.to_string()),
        );
        syn::visit::visit_item_struct(self, node);
    }

    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.push(
            node.span().start(),
            node.span().end(),
            "enum",
            Some(node.ident.to_string()),
        );
        syn::visit::visit_item_enum(self, node);
    }

    fn visit_item_union(&mut self, node: &'ast syn::ItemUnion) {
        self.push(
            node.span().start(),
            node.span().end(),
            "union",
            Some(node.ident.to_string()),
        );
        syn::visit::visit_item_union(self, node);
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        self.push(
            node.span().start(),
            node.span().end(),
            "const",
            Some(node.ident.to_string()),
        );
        syn::visit::visit_item_const(self, node);
    }

    fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
        self.push(
            node.span().start(),
            node.span().end(),
            "static",
            Some(node.ident.to_string()),
        );
        syn::visit::visit_item_static(self, node);
    }

    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        self.push(
            node.span().start(),
            node.span().end(),
            "type",
            Some(node.ident.to_string()),
        );
        syn::visit::visit_item_type(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.push(
            node.span().start(),
            node.span().end(),
            "trait",
            Some(node.ident.to_string()),
        );
        syn::visit::visit_item_trait(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        self.push(node.span().start(), node.span().end(), "impl", None);
        syn::visit::visit_item_impl(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.push(
            node.span().start(),
            node.span().end(),
            "module",
            Some(node.ident.to_string()),
        );
        syn::visit::visit_item_mod(self, node);
    }

    fn visit_item_macro(&mut self, node: &'ast syn::ItemMacro) {
        self.push(
            node.span().start(),
            node.span().end(),
            "macro",
            node.ident.as_ref().map(ToString::to_string),
        );
        syn::visit::visit_item_macro(self, node);
    }
}

pub fn group_hunks(
    hunks: &[TextHunk],
    old_sources: &BTreeMap<String, String>,
    new_sources: &BTreeMap<String, String>,
) -> Vec<SyntaxEditGroup> {
    let old_regions = parse_regions(old_sources);
    let new_regions = parse_regions(new_sources);
    let mut grouped = BTreeMap::<String, Vec<TextHunk>>::new();
    let mut metadata = BTreeMap::<String, (String, String, Option<String>)>::new();

    for hunk in hunks {
        let summary = &hunk.summary;
        let region = new_regions
            .get(&summary.file)
            .and_then(|regions| enclosing(regions, summary.new_start, summary.new_lines))
            .or_else(|| {
                old_regions
                    .get(&summary.file)
                    .and_then(|regions| enclosing(regions, summary.old_start, summary.old_lines))
            });
        let (key, language, kind, symbol) = region.map_or_else(
            || {
                (
                    format!("text:{}:{}", summary.file, summary.id),
                    "text".to_owned(),
                    "hunk".to_owned(),
                    None,
                )
            },
            |region| {
                (
                    format!(
                        "rust:{}:{}:{}:{}",
                        summary.file, region.kind, region.start, region.end
                    ),
                    "rust".to_owned(),
                    region.kind.to_owned(),
                    region.symbol.clone(),
                )
            },
        );
        metadata
            .entry(key.clone())
            .or_insert((language, kind, symbol));
        grouped.entry(key).or_default().push(hunk.clone());
    }

    grouped
        .into_iter()
        .map(|(key, mut members)| {
            members.sort_by_key(|hunk| {
                (
                    hunk.summary.file.clone(),
                    hunk.summary.old_start,
                    hunk.summary.new_start,
                )
            });
            let (language, kind, symbol) = metadata
                .remove(&key)
                .expect("group metadata is inserted with every group");
            let mut digest = Sha256::new();
            digest.update(key.as_bytes());
            for hunk in &members {
                digest.update(hunk.summary.id.as_bytes());
            }
            let id = format!("syntax.{}", crate::hex_prefix(&digest.finalize(), 8));
            let file = members[0].summary.file.clone();
            SyntaxEditGroup {
                summary: SyntaxEditGroupSummary {
                    id,
                    language,
                    kind,
                    symbol,
                    file,
                    member_hunks: members.iter().map(|hunk| hunk.summary.id.clone()).collect(),
                },
                hunks: members,
            }
        })
        .collect()
}

fn parse_regions(sources: &BTreeMap<String, String>) -> BTreeMap<String, Vec<Region>> {
    sources
        .iter()
        .filter(|(path, _)| {
            std::path::Path::new(path)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
        })
        .filter_map(|(path, source)| {
            let syntax = syn::parse_file(source).ok()?;
            let mut collector = RegionCollector::default();
            collector.visit_file(&syntax);
            Some((path.clone(), collector.regions))
        })
        .collect()
}

fn enclosing(regions: &[Region], start: usize, line_count: usize) -> Option<&Region> {
    let end = start.saturating_add(line_count.saturating_sub(1));
    regions
        .iter()
        .filter(|region| region.start <= start && region.end >= end)
        .min_by_key(|region| region.end.saturating_sub(region.start))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::TextHunkSummary;

    fn hunk(id: &str, line: usize) -> TextHunk {
        TextHunk {
            summary: TextHunkSummary {
                id: id.to_owned(),
                parent_atom: "file.0123456789abcdef".to_owned(),
                file: "src/lib.rs".to_owned(),
                old_start: line,
                old_lines: 1,
                new_start: line,
                new_lines: 1,
                removed_preview: Vec::new(),
                added_preview: Vec::new(),
            },
            header: Vec::new(),
            patch: Vec::new(),
        }
    }

    #[test]
    fn groups_separated_hunks_inside_one_rust_function() {
        let source =
            "fn changed() {\n    let first = 1;\n\n    let second = 2;\n}\n\nfn other() {}\n";
        let sources = BTreeMap::from([("src/lib.rs".to_owned(), source.to_owned())]);
        let groups = group_hunks(
            &[
                hunk("hunk.0000000000000001", 2),
                hunk("hunk.0000000000000002", 4),
                hunk("hunk.0000000000000003", 7),
            ],
            &sources,
            &sources,
        );
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].summary.member_hunks.len(), 2);
        assert_eq!(groups[0].summary.kind, "function");
        assert_eq!(groups[0].summary.symbol.as_deref(), Some("changed"));
    }

    #[test]
    fn malformed_rust_uses_explicit_one_hunk_fallback_groups() {
        let source = BTreeMap::from([("src/lib.rs".to_owned(), "fn incomplete( {".to_owned())]);
        let groups = group_hunks(
            &[
                hunk("hunk.0000000000000001", 1),
                hunk("hunk.0000000000000002", 2),
            ],
            &source,
            &source,
        );
        assert_eq!(groups.len(), 2);
        assert!(groups.iter().all(|group| group.summary.language == "text"));
        assert!(
            groups
                .iter()
                .all(|group| group.summary.member_hunks.len() == 1)
        );
    }
}
