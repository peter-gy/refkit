---
description: Inspect duplicate evidence, resolve field conflicts, and apply an accepted merge plan in Python or TypeScript.
---

# Review Duplicate References

`BibDocument.find_duplicates()` / `findDuplicates()` returns candidate groups, rule evidence, and conflicting values. `plan_merge()` / `planMerge()` turns your retained-entry and field choices into an inspectable patch. Both calls leave the source unchanged.

## Inspect a candidate group

::: code-group

```python [Python]
import refkit as rk

source = """@string{press={Example Press}}
@book{a,title={First Title},author={Doe, Jane},year=2024,doi={10.1234/work}}
@book{b,title={Revised Title},doi={10.1234/work},publisher=press}
@misc{child,crossref={b}}
"""
document = rk.BibDocument.parse(source)
report = document.find_duplicates(rules=["doi"])
group = report["groups"][0]
print([member["key"] for member in group["members"]])
print([conflict["field"] for conflict in group["conflicts"]])
```

```ts [TypeScript]
import * as rk from "refkit-js";

const source = `@string{press={Example Press}}
@book{a,title={First Title},author={Doe, Jane},year=2024,doi={10.1234/work}}
@book{b,title={Revised Title},doi={10.1234/work},publisher=press}
@misc{child,crossref={b}}
`;
const document = rk.BibDocument.parse(source);
const report = document.findDuplicates({ rules: ["doi"] });
const group = report.groups[0]!;
console.log(group.members.map((member) => member.key));
console.log(group.conflicts.map((conflict) => conflict.field));
```

:::

The group contains `a` and `b`, with a conflict for `title`. Each value names its source entry and field occurrence and includes the complete BibTeX `expression`. Group IDs and member IDs belong to this input snapshot.

## Choose values and inspect the plan

::: code-group

```python [Python]
members = [member["entry_id"] for member in group["members"]]
pending = document.plan_merge(members, retain=0)
assert pending["patch"] is None

title = document.entries["b"].fields["title"]
plan = document.plan_merge(
    members,
    retain=0,
    fields=[
        {
            "kind": "take",
            "name": "title",
            "entry_id": title.entry_id,
            "field_id": title.id,
        }
    ],
)
patch = plan["patch"]
assert patch is not None
updated = document.apply_patch(patch)["document"]
print(updated.resolve()[0]["fields"]["publisher"])
print(updated.resolve()[1]["fields"]["crossref"])
```

```ts [TypeScript]
const members = group.members.map((member) => member.entryId);
const pending = document.planMerge({ entries: members, retain: 0 });
if (pending.patch !== null) throw new Error("Expected a title conflict");

const title = document.entries.getUnique("b")!.fields.getUnique("title")!;
const plan = document.planMerge({ entries: members, retain: 0, fields: [{
  kind: "take", name: "title", entryId: title.entryId, fieldId: title.id,
}] });
if (plan.patch === null) throw new Error("Resolve remaining conflicts");
const updated = document.applyPatch(plan.patch).document;
console.log(updated.resolve()[0]!.fields.publisher);
console.log(updated.resolve()[1]!.fields.crossref);
```

:::

Both print `Example Press` and `a`. The publisher remains a macro expression in the source, and the child's reference points to the retained entry. The original document and its handles remain unchanged. Inspect `plan.patch` before applying it, then review the resulting [byte changes and occurrence mappings](/reference/data-shapes#patch-reports).

A field with one distinct source expression is selected automatically, preferring the retained entry. Missing retained fields are copied from the selection. Different expressions require a `take` choice naming a source occurrence or a `drop` choice naming the field. One value per field is retained, including when the source contains duplicate field occurrences. Entry-type conflicts require `entry_type` / `entryType` explicitly.

An unresolved plan has a null `patch` and lists its remaining `conflicts`. Invalid choices, ambiguous reference rewrites, and reference cycles raise `MergeError`. A ready plan has been checked by the atomic patch engine. Apply it to the same input snapshot whose IDs it contains.

Plans rewrite `crossref`, `xdata`, and `xref`. Referenced keys must identify at most one surviving entry. Cycle checks apply to inheritance through `crossref` and `xdata`. An `xref` is a related-entry link, so reciprocal `xref` links remain allowed.

## Understand matching evidence

The default rules are `doi`, `key`, `abstract`, and `citation`, in that order. An empty rule list returns no groups. Repeated rules are ignored. Entries connected through any selected rule belong to one group.

| Rule | Signature used by review and tidy |
| --- | --- |
| `key` | ASCII-lowercased citation key. |
| `doi` | First DOI field with nonalphanumeric characters removed and remaining characters lowercased. |
| `abstract` | First 100 characters of the similarly normalized first abstract field. |
| `citation` | Normalized first-author surname, title, and number. Missing number uses `0`. |

These are candidate signatures. For example, the DOI rule can group `10.1234/a-b` with `10.1234/ab`. Distinct valid canonical identifier strings are exposed as identifier conflicts, and a field choice is required before a merge patch is produced. Source-expression differences can also require a choice when their rendered text looks similar.

Review and planning accept sources up to 16 MiB. Merge reference graphs are bounded at 100,000 edges, and generated plans use the [patch operation and output limits](/guides/edit-bibtex#apply-structural-changes-atomically).

Use [Format BibTeX](/guides/format-bibtex) when deterministic formatter merge settings fit the workflow. Use review plans when a person or application needs to choose the retained entry and individual fields.
