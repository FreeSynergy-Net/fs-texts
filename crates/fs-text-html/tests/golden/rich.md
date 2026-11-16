---
title: Rich page
description: Every construct once.
---
# Rich page

Intro with *emphasis*, **strong**, ~~strike~~, `code`, a [link](https://example.org/a?b=1&c=2 "T"), and a note.[^n]
Hard break here  
and a keyboard key <kbd>Ctrl</kbd> and <script>bad()</script>.

## Lists

- one
- two
  1. nested ordered
  2. second

3. starts at three
4. four

- [ ] open task
- [x] done task

## Quote and callouts

> A quote.

> [!warning] Mind this
> Callout body with `code`.

> [!tip]- Folded
> Hidden text.

## Table

| Left | Centre | Right |
|:-----|:------:|------:|
| a | **b** | c |
| [ ] | [x] | `d` |

## Code

```rust
fn main() { println!("<b>"); }
```

<details lang="en" onclick="x()">
<summary>More</summary>

Inside.

</details>

![A picture](pic.png)

---

[^n]: The footnote with *marks*.
