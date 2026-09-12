---
type: Decision
title: Compose native formats from framing, document codecs, and profiles
description: Native format composition and the choice of a closed format catalog.
generated: { by: codex/gpt-6-astra, at: 2026-09-12T07:43:11Z }
---

# Compose native formats from framing, document codecs, and profiles

tq models each native format as a composition of a document format, framing, and directional format profiles. Framing owns sequence headers and frame boundaries; document codecs own the syntax-to-value mapping for one document; a format catalog owns names, aliases, extensions, capabilities, and compatible options. The catalog and adapters keep closed Rust enums and exhaustive matching because tq has no third-party format extension requirement, while a dynamic registry would weaken those checks without removing format-specific semantics.
