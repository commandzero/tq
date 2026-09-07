---
type: Policy
title: "Changelog policy"
description: "How tq records notable changes and preserves release history."
generated: { by: codex/gpt-6, at: 2026-09-07T01:44:50Z }
---

# Changelog policy

Use this policy for new entries. Preserve historical release content and ordering.
This repository uses the shared updates guidance for new entries.

1. Keep an Unreleased section, dated version headings, and comparison links.
2. Use Security, Removed, Changed, Deprecated, Fixed, Added in that order.
   Omit empty categories.
3. Use Fixed when previous behavior was wrong. A regression test or code evidence
   can establish a fix; an issue number or published release note is not required.
4. Use Changed when intentional behavior changes. Mark breaking changes within
   their category and give an upgrade action.
5. Describe notable user effects. Internal maintenance does not require an entry.
6. Issue and PR links are optional. Add only verified links; never invent references.
7. Derive release notes from the curated version section. A maintainer reviews
   notable changes before publication.

The shared bundle contains a supplied Keep a Changelog 2.0.0 snapshot. These are
local repository rules; they do not assert that version is published online.
The local changelog skill points here to avoid a second policy copy.
