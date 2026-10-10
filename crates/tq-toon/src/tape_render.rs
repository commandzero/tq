use std::io::Write;
use std::sync::Arc;

use super::{
    ArrayCursor, ColorPalette, FieldSchema, PreparationMemory, PreparedNode, PreparedTape,
    RowSchema, ScalarContext, TapeError, WriterConfig, writer,
};

const MAX_SCHEMA_DEPTH: usize = 256;
const MAX_SCHEMA_FIELDS: usize = 65_536;
const MAX_SCHEMA_BYTES: usize = 16 * 1024 * 1024;

struct RenderOutput<'a, W> {
    out: &'a mut W,
    cfg: WriterConfig,
    palette: Option<&'a ColorPalette>,
    wrote_line: &'a mut bool,
}

impl PreparedTape {
    pub(crate) fn write<W: Write>(
        &mut self,
        node: &PreparedNode,
        out: &mut W,
        cfg: WriterConfig,
        palette: Option<&ColorPalette>,
    ) -> Result<(), TapeError> {
        let mut wrote_line = false;
        self.render_value(
            node.index,
            None,
            None,
            0,
            true,
            true,
            false,
            out,
            cfg,
            palette,
            &mut wrote_line,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_value<W: Write>(
        &mut self,
        index: u64,
        key: Option<&str>,
        prefix: Option<&'static str>,
        depth: usize,
        eligible: bool,
        root: bool,
        line_started: bool,
        out: &mut W,
        cfg: WriterConfig,
        palette: Option<&ColorPalette>,
        wrote_line: &mut bool,
    ) -> Result<(), TapeError> {
        if depth > MAX_SCHEMA_DEPTH {
            return Err(TapeError::Structure("maximum replay depth exceeded"));
        }
        let (tag, count) = self.header(index)?;
        match tag {
            0..=4 => {
                if !line_started {
                    Self::line_start(out, depth, prefix, wrote_line, cfg, palette)?;
                }
                if let Some(key) = key {
                    writer::write_key(out, key, palette)?;
                    out.write_all(b": ")?;
                }
                self.scalar_render(
                    index,
                    out,
                    cfg,
                    palette,
                    if root {
                        ScalarContext::Root
                    } else if key.is_some() {
                        ScalarContext::Object
                    } else {
                        ScalarContext::Array
                    },
                )?;
            }
            5 => self.render_array(
                index,
                count,
                key,
                prefix,
                depth,
                eligible,
                root,
                line_started,
                out,
                cfg,
                palette,
                wrote_line,
            )?,
            6 => self.render_object(
                index,
                count,
                key,
                prefix,
                depth,
                eligible,
                root,
                line_started,
                out,
                cfg,
                palette,
                wrote_line,
            )?,
            _ => return Err(TapeError::Structure("invalid prepared node tag")),
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn render_object<W: Write>(
        &mut self,
        index: u64,
        count: u64,
        key: Option<&str>,
        prefix: Option<&'static str>,
        depth: usize,
        eligible: bool,
        root: bool,
        line_started: bool,
        out: &mut W,
        cfg: WriterConfig,
        palette: Option<&ColorPalette>,
        wrote_line: &mut bool,
    ) -> Result<(), TapeError> {
        if eligible && (key.is_some() || root) && count >= 2 {
            let cursor = self.object_cursor(index, count)?;
            let mut output = RenderOutput {
                out,
                cfg,
                palette,
                wrote_line,
            };
            if self.render_keyed_object(cursor, key, prefix, depth, line_started, &mut output)? {
                return Ok(());
            }
        }
        let fields = self.object_info(index)?;

        if fields.is_empty() {
            if !line_started && (key.is_some() || prefix.is_some()) {
                Self::line_start(
                    out,
                    depth,
                    prefix.map(|value| value.trim_end_matches(' ')),
                    wrote_line,
                    cfg,
                    palette,
                )?;
            }
            if let Some(key) = key {
                writer::write_key(out, key, palette)?;
                out.write_all(b":")?;
            }
            return Ok(());
        }

        let list_item = key.is_none() && prefix == Some("- ");
        if !line_started && (key.is_some() || prefix.is_some()) {
            Self::line_start(out, depth, prefix, wrote_line, cfg, palette)?;
        }
        if let Some(key) = key {
            writer::write_key(out, key, palette)?;
            out.write_all(b":")?;
        }
        let content_depth = depth + usize::from(key.is_some()) + usize::from(prefix.is_some());
        for (position, (field_key, child)) in fields.iter().enumerate() {
            let inline_first = list_item && position == 0;
            self.render_value(
                *child,
                Some(field_key),
                None,
                content_depth,
                true,
                false,
                inline_first,
                out,
                cfg,
                palette,
                wrote_line,
            )?;
        }
        Ok(())
    }

    fn render_keyed_object<W: Write>(
        &mut self,
        mut cursor: super::ObjectCursor,
        key: Option<&str>,
        prefix: Option<&'static str>,
        depth: usize,
        line_started: bool,
        output: &mut RenderOutput<'_, W>,
    ) -> Result<bool, TapeError> {
        let first_child = self
            .object_cursor_next(&mut cursor)?
            .ok_or(TapeError::Structure("keyed object has no first entry"))?
            .child;
        let mut charge = self.arena.memory_charge();
        let mut schema_bytes = std::mem::size_of::<RowSchema>();
        charge.grow(schema_bytes)?;
        let mut field_count = 0;
        let Some(schema) = self.scalar_row_schema(
            first_child,
            &mut charge,
            1,
            &mut field_count,
            &mut schema_bytes,
        )?
        else {
            return Ok(false);
        };
        while let Some(entry) = self.object_cursor_next(&mut cursor)? {
            if !self.matches_row_schema(entry.child, &schema)? {
                return Ok(false);
            }
        }
        if !line_started {
            Self::line_start(
                output.out,
                depth,
                prefix,
                output.wrote_line,
                output.cfg,
                output.palette,
            )?;
        }
        writer::write_table_header_colored(
            output.out,
            key,
            usize::try_from(cursor.count).map_err(|_| super::SpoolError::Limit)?,
            true,
            Some(&schema),
            output.cfg,
            output.palette,
        )?;
        let content_depth = depth + 1 + usize::from(prefix.is_some());
        cursor.rewind();
        while let Some(entry) = self.object_cursor_next(&mut cursor)? {
            Self::line_start(
                output.out,
                content_depth,
                None,
                output.wrote_line,
                output.cfg,
                output.palette,
            )?;
            self.write_object_entry_key(&entry, output.out, output.palette)?;
            output.out.write_all(b": ")?;
            let mut first = true;
            self.write_scalar_row(
                entry.child,
                &schema,
                output.out,
                output.cfg,
                output.palette,
                &mut first,
            )?;
        }
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_array<W: Write>(
        &mut self,
        index: u64,
        count: u64,
        key: Option<&str>,
        prefix: Option<&'static str>,
        depth: usize,
        eligible: bool,
        root: bool,
        line_started: bool,
        out: &mut W,
        cfg: WriterConfig,
        palette: Option<&ColorPalette>,
        wrote_line: &mut bool,
    ) -> Result<(), TapeError> {
        if !line_started {
            Self::line_start(out, depth, prefix, wrote_line, cfg, palette)?;
        }
        if count == 0 {
            if key.is_some() || prefix.is_none() {
                if let Some(key) = key {
                    writer::write_key(out, key, palette)?;
                    out.write_all(b": ")?;
                }
                tq_core::presentation::write_span(
                    out,
                    palette,
                    tq_core::presentation::ColorRole::Array,
                    b"[]",
                )?;
            } else {
                writer::write_table_header_colored(out, None, 0, false, None, cfg, palette)?;
            }
            return Ok(());
        }

        let native_count = usize::try_from(count)
            .map_err(|_| TapeError::Structure("array count exceeds platform limits"))?;
        let mut cursor = self.array_cursor(index)?;
        let all_scalar = self.array_all_scalars(&mut cursor)?;
        let tabular_allowed = eligible && (key.is_some() || root);
        let mut schema_charge = self.arena.memory_charge();
        let schema = if tabular_allowed && !all_scalar {
            let mut schema_bytes = std::mem::size_of::<RowSchema>();
            schema_charge.grow(schema_bytes)?;
            let mut field_count = 0;
            let first = self.array_cursor_child(&mut cursor, 0)?;
            self.scalar_row_schema(
                first,
                &mut schema_charge,
                1,
                &mut field_count,
                &mut schema_bytes,
            )?
        } else {
            None
        };
        let mut tabular = schema.is_some();
        if let Some(schema) = schema.as_ref() {
            for position in 1..count {
                let child = self.array_cursor_child(&mut cursor, position)?;
                if !self.matches_row_schema(child, schema)? {
                    tabular = false;
                    break;
                }
            }
        }

        let mut output = RenderOutput {
            out,
            cfg,
            palette,
            wrote_line,
        };
        if all_scalar {
            return self.render_scalar_array(&mut cursor, count, native_count, key, &mut output);
        }
        if tabular {
            let schema = schema
                .as_ref()
                .ok_or(TapeError::Structure("missing row schema"))?;
            writer::write_table_header_colored(
                output.out,
                key,
                native_count,
                false,
                Some(schema),
                output.cfg,
                output.palette,
            )?;
            return self.render_tabular_array(&mut cursor, count, depth, schema, &mut output);
        }

        drop(schema);
        drop(schema_charge);
        self.render_nested_array(&mut cursor, count, native_count, key, depth, &mut output)
    }

    fn render_scalar_array<W: Write>(
        &mut self,
        cursor: &mut ArrayCursor,
        count: u64,
        native_count: usize,
        key: Option<&str>,
        output: &mut RenderOutput<'_, W>,
    ) -> Result<(), TapeError> {
        writer::write_table_header_colored(
            output.out,
            key,
            native_count,
            false,
            None,
            output.cfg,
            output.palette,
        )?;
        output.out.write_all(b" ")?;
        for position in 0..count {
            if position != 0 {
                Self::write_delimiter(
                    output.out,
                    output.cfg,
                    output.palette,
                    tq_core::presentation::ColorRole::Array,
                )?;
            }
            let child = self.array_cursor_child(cursor, position)?;
            self.scalar_render(
                child,
                output.out,
                output.cfg,
                output.palette,
                ScalarContext::Array,
            )?;
        }
        Ok(())
    }

    fn render_tabular_array<W: Write>(
        &mut self,
        cursor: &mut ArrayCursor,
        count: u64,
        depth: usize,
        schema: &RowSchema,
        output: &mut RenderOutput<'_, W>,
    ) -> Result<(), TapeError> {
        let row_depth = depth + 1;
        for position in 0..count {
            let child = self.array_cursor_child(cursor, position)?;
            Self::line_start(
                output.out,
                row_depth,
                None,
                output.wrote_line,
                output.cfg,
                output.palette,
            )?;
            let mut first = true;
            self.write_scalar_row(
                child,
                schema,
                output.out,
                output.cfg,
                output.palette,
                &mut first,
            )?;
        }
        Ok(())
    }

    fn render_nested_array<W: Write>(
        &mut self,
        cursor: &mut ArrayCursor,
        count: u64,
        native_count: usize,
        key: Option<&str>,
        depth: usize,
        output: &mut RenderOutput<'_, W>,
    ) -> Result<(), TapeError> {
        writer::write_table_header_colored(
            output.out,
            key,
            native_count,
            false,
            None,
            output.cfg,
            output.palette,
        )?;
        let item_depth = depth + 1;
        for position in 0..count {
            let child = self.array_cursor_child(cursor, position)?;
            let (child_tag, child_count) = self.header(child)?;
            let item_prefix = if child_tag == 6 && child_count == 0 {
                "-"
            } else {
                "- "
            };
            self.render_value(
                child,
                None,
                Some(item_prefix),
                item_depth,
                false,
                false,
                false,
                output.out,
                output.cfg,
                output.palette,
                output.wrote_line,
            )?;
        }
        Ok(())
    }

    fn scalar_row_schema(
        &mut self,
        index: u64,
        charge: &mut PreparationMemory,
        depth: usize,
        field_count: &mut usize,
        schema_bytes: &mut usize,
    ) -> Result<Option<RowSchema>, TapeError> {
        if self.header(index)?.0 != 6 {
            return Ok(None);
        }
        if depth > MAX_SCHEMA_DEPTH {
            return Err(crate::schema::SchemaError::Depth(MAX_SCHEMA_DEPTH).into());
        }
        let fields = self.object_info(index)?;
        if fields.is_empty() {
            return Ok(None);
        }
        *field_count = field_count
            .checked_add(fields.len())
            .filter(|count| *count <= MAX_SCHEMA_FIELDS)
            .ok_or(crate::schema::SchemaError::Fields(MAX_SCHEMA_FIELDS))?;
        let bytes = fields
            .len()
            .checked_mul(std::mem::size_of::<FieldSchema>())
            .ok_or(crate::schema::SchemaError::Bytes(MAX_SCHEMA_BYTES))?;
        *schema_bytes = schema_bytes
            .checked_add(bytes)
            .filter(|bytes| *bytes <= MAX_SCHEMA_BYTES)
            .ok_or(crate::schema::SchemaError::Bytes(MAX_SCHEMA_BYTES))?;
        charge.grow(bytes)?;
        let mut schema = Vec::with_capacity(fields.len());
        for (key, child) in fields.iter() {
            let key_bytes = crate::schema::retained_key_bytes(key);
            *schema_bytes = schema_bytes
                .checked_add(key_bytes)
                .filter(|bytes| *bytes <= MAX_SCHEMA_BYTES)
                .ok_or(crate::schema::SchemaError::Bytes(MAX_SCHEMA_BYTES))?;
            charge.grow(key_bytes)?;
            let tag = self.header(*child)?.0;
            let children = if tag == 6 {
                let Some(children) =
                    self.scalar_row_schema(*child, charge, depth + 1, field_count, schema_bytes)?
                else {
                    return Ok(None);
                };
                Some(children)
            } else if tag <= 4 {
                None
            } else {
                return Ok(None);
            };
            schema.push(FieldSchema {
                key: Arc::from(key.as_str()),
                children,
            });
        }
        Ok(Some(RowSchema { fields: schema }))
    }

    fn matches_row_schema(&mut self, index: u64, schema: &RowSchema) -> Result<bool, TapeError> {
        if self.header(index)?.0 != 6 {
            return Ok(false);
        }
        if schema.fields.len() <= 8 {
            let mut charge = self.arena.memory_charge();
            charge.grow(8 * std::mem::size_of::<u64>())?;
            let mut children = [0_u64; 8];
            if self.ordered_object_children(index, schema, &mut children)? {
                return self.matches_ordered_children(schema, &children[..schema.fields.len()]);
            }
        }
        self.matches_row_schema_fallback(index, schema)
    }

    fn matches_ordered_children(
        &mut self,
        schema: &RowSchema,
        children: &[u64],
    ) -> Result<bool, TapeError> {
        for (field, child) in schema.fields.iter().zip(children) {
            if let Some(children) = &field.children {
                if !self.matches_row_schema(*child, children)? {
                    return Ok(false);
                }
            } else if self.header(*child)?.0 > 4 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn matches_row_schema_fallback(
        &mut self,
        index: u64,
        schema: &RowSchema,
    ) -> Result<bool, TapeError> {
        if self.header(index)?.0 != 6 {
            return Ok(false);
        }
        let mut fields = self.object_info(index)?;
        if fields.len() != schema.fields.len() {
            return Ok(false);
        }
        fields
            .fields
            .sort_unstable_by(|left, right| left.0.cmp(&right.0));
        for field in &schema.fields {
            let Ok(position) =
                fields.binary_search_by(|(key, _)| key.as_str().cmp(field.key.as_ref()))
            else {
                return Ok(false);
            };
            let child = fields[position].1;
            if let Some(children) = &field.children {
                if !self.matches_row_schema(child, children)? {
                    return Ok(false);
                }
            } else if self.header(child)?.0 > 4 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn write_scalar_row<W: Write>(
        &mut self,
        index: u64,
        schema: &RowSchema,
        out: &mut W,
        cfg: WriterConfig,
        palette: Option<&ColorPalette>,
        first: &mut bool,
    ) -> Result<(), TapeError> {
        if schema.fields.len() <= 8 {
            let mut charge = self.arena.memory_charge();
            charge.grow(8 * std::mem::size_of::<u64>())?;
            let mut children = [0_u64; 8];
            if self.ordered_object_children(index, schema, &mut children)? {
                return self.write_ordered_scalar_row(
                    schema,
                    &children[..schema.fields.len()],
                    out,
                    cfg,
                    palette,
                    first,
                );
            }
        }
        self.write_scalar_row_fallback(index, schema, out, cfg, palette, first)
    }

    fn write_ordered_scalar_row<W: Write>(
        &mut self,
        schema: &RowSchema,
        children: &[u64],
        out: &mut W,
        cfg: WriterConfig,
        palette: Option<&ColorPalette>,
        first: &mut bool,
    ) -> Result<(), TapeError> {
        for (field, child) in schema.fields.iter().zip(children) {
            if let Some(children_schema) = &field.children {
                self.write_scalar_row(*child, children_schema, out, cfg, palette, first)?;
            } else {
                if !*first {
                    Self::write_delimiter(
                        out,
                        cfg,
                        palette,
                        tq_core::presentation::ColorRole::Object,
                    )?;
                }
                self.scalar_render(*child, out, cfg, palette, ScalarContext::Object)?;
                *first = false;
            }
        }
        Ok(())
    }

    fn write_scalar_row_fallback<W: Write>(
        &mut self,
        index: u64,
        schema: &RowSchema,
        out: &mut W,
        cfg: WriterConfig,
        palette: Option<&ColorPalette>,
        first: &mut bool,
    ) -> Result<(), TapeError> {
        let mut fields = self.object_info(index)?;
        fields
            .fields
            .sort_unstable_by(|left, right| left.0.cmp(&right.0));
        for field in &schema.fields {
            let position = fields
                .binary_search_by(|(key, _)| key.as_str().cmp(field.key.as_ref()))
                .map_err(|_| TapeError::Structure("row field missing"))?;
            let child = fields[position].1;
            if let Some(children) = &field.children {
                self.write_scalar_row(child, children, out, cfg, palette, first)?;
            } else {
                if !*first {
                    Self::write_delimiter(
                        out,
                        cfg,
                        palette,
                        tq_core::presentation::ColorRole::Object,
                    )?;
                }
                self.scalar_render(child, out, cfg, palette, ScalarContext::Object)?;
                *first = false;
            }
        }
        Ok(())
    }

    fn array_all_scalars(&mut self, cursor: &mut ArrayCursor) -> Result<bool, TapeError> {
        for position in 0..cursor.count() {
            let child = self.array_cursor_child(cursor, position)?;
            if self.header(child)?.0 > 4 {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn line_start<W: Write>(
        out: &mut W,
        depth: usize,
        prefix: Option<&'static str>,
        wrote_line: &mut bool,
        cfg: WriterConfig,
        palette: Option<&ColorPalette>,
    ) -> Result<(), TapeError> {
        if *wrote_line {
            out.write_all(b"\n")?;
        }
        *wrote_line = true;
        for _ in 0..depth.saturating_mul(cfg.indent_size) {
            out.write_all(b" ")?;
        }
        if let Some(prefix) = prefix {
            tq_core::presentation::write_span(
                out,
                palette,
                tq_core::presentation::ColorRole::Array,
                b"-",
            )?;
            if prefix == "- " {
                out.write_all(b" ")?;
            }
        }
        Ok(())
    }

    fn write_delimiter<W: Write>(
        out: &mut W,
        cfg: WriterConfig,
        palette: Option<&ColorPalette>,
        role: tq_core::presentation::ColorRole,
    ) -> Result<(), TapeError> {
        let mut delimiter = [0; 4];
        tq_core::presentation::write_span(
            out,
            palette,
            role,
            cfg.delimiter
                .character()
                .encode_utf8(&mut delimiter)
                .as_bytes(),
        )?;
        Ok(())
    }
}
