# Appkit

> **★★★ CSE / Knowable Construction.** This repo operates under **Constructive Substrate Engineering** — canonical specification at [`pleme-io/theory/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md`](https://github.com/pleme-io/theory/blob/main/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md). The Compounding Directive (operational rules: solve once, load-bearing fixes only, idiom-first, models stay current, direction beats velocity) is in the org-level pleme-io/CLAUDE.md ★★★ section. Read both before non-trivial changes.

Shared GPU application bootstrap for pleme-io. Extracts the ~800 LOC of boilerplate that every GPU app (mado, hibiki, kagi, fumi, nami, hikyaku, tobirato, ayatsuri, hikki) was copying into its own `main.rs`:
