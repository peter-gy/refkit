---
description: Filter normalized Library entries by type, fields, alternatives, and parent structure.
---

# Selectors

`Library.select(selector)` filters normalized entries with the [Hayagriva selector language](https://github.com/typst/hayagriva/blob/main/docs/selectors.md). Selectors match entry types, fields, and parent structure.

## Match an entry type

```python
articles = library.select("article")
everything = library.select("*")
```

String selectors are case-insensitive. The entry type must be part of Hayagriva's bibliography model.

## Require fields

```python
dated_articles = library.select("article[date]")
complete_articles = library.select("article[author,title,date]")
```

Every field listed inside brackets must contain a value.

## Match parent structure

```python
periodical_articles = library.select("article > periodical[volume]")
```

`>` requires a matching parent. It can be chained to inspect deeper parent relationships.

## Combine conditions

```python
media_articles = library.select("article > (conference & video)")
books_or_articles = library.select("book | article")
not_books = library.select("!book")
```

`|` selects alternatives. `&` requires multiple matching parents on the right side of `>`. `!` negates the next selector. Parentheses group expressions.

RefKit returns the top-level `Entry` objects that match. Selector bindings remain an internal part of matching and are not returned by `Library.select`.

An invalid selector raises `ValueError` before any result list is returned.
