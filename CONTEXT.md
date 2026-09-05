# tq

tq applies jq-compatible queries to structured input and writes ordered results. Its shared language distinguishes input bytes, decoded values, query results, and structural events.

## Query model

**Query**:
A jq-compatible expression that maps each input value to an ordered sequence of zero or more results.
_Avoid_: Filter, program

**Value**:
An ordered JSON-shaped datum: null, boolean, number, string, array, or insertion-ordered object. A value may be a document root, an object member, an array element, or a query result.
_Avoid_: DOM, node

**Complete value**:
A value whose entire representation within its document has been consumed. Before completion, tq has structural events and decoder state rather than a partial value.
_Avoid_: Partial value, subtree

**Input value**:
A value supplied as one invocation input to a query. One source may yield zero, one, or many input values.
_Avoid_: Item

**Input sequence**:
All input values in source and frame order.
_Avoid_: Whole input, input stream

**Input-sequence access**:
A query's ability to consume later input values through `input` or `inputs` without requiring them all to be retained.
_Avoid_: Whole-input access, slurp

**Slurp mode**:
The jq-compatible input mode that collects the complete input sequence into one array input value before running a query.
_Avoid_: Input-sequence access, whole-input mode

**Result**:
A value yielded by a query. Native structured output maps each result to exactly one document and one frame.
_Avoid_: Output, response

## Query planning

**Query requirement**:
A semantic need that every correct execution of a query must satisfy.
_Avoid_: Capability, plan eligibility

**Plan eligibility**:
The proven conditions under which an execution plan can preserve query semantics for the selected input and output.
_Avoid_: Query requirement, capability

**Execution plan**:
The selected way to run a query for the chosen native formats, profiles, input modes, and output commitment.
_Avoid_: Query, capability

**Plan proof**:
Recorded evidence that an execution plan satisfies the query requirements before semantic input is consumed.
_Avoid_: Heuristic, optimizer hint

**Capability**:
Declared support for a feature by tq, a platform, or a native format, independent of one query's requirements or plan eligibility.
_Avoid_: Query requirement, plan eligibility

**Blocking requirement**:
A query requirement where correct result order depends on completing a relevant finite collection.
_Avoid_: Blocking plan, complete-document requirement

**Retention scope**:
The active structure, complete values, sequence context, and query state an execution plan must retain at once to satisfy every query requirement.
_Avoid_: Working set, whole input

## Input and output

**Source**:
An origin of input bytes, such as standard input or a named file. Each source has its own format selection and error outcome.
_Avoid_: Input file

**Document**:
A complete top-level unit in a native format. Each document has exactly one root value and is represented by one frame.
_Avoid_: Record, input value

**Root value**:
The value at the top level of a document. A root value may be a scalar, array, or object.
_Avoid_: Document object, payload

**Document sequence**:
The ordered documents represented by a source's frames. Each document remains a separate top-level query input unless an input mode combines or transforms the sequence.
_Avoid_: Record stream, input stream

**Committed native input**:
The processing of one source after format commitment. It applies the selected framing and input profile to yield ordered native input observations.
_Avoid_: Parser session, decoder pipeline

**Native input observation**:
An ordered document, structural event, or recoverable document failure yielded by committed native input. Callers decide how to evaluate documents or events and where to render failures.
_Avoid_: Parser callback, warning side effect

**Native output sequence**:
The mapping of all structured results for one command output into one document sequence under a selected framing and output profile. It retains sequence context across input sources and intervening proxy bytes, and defines output commitment through completion.
_Avoid_: Writer session, output stream

**Command output**:
The complete ordered stdout bytes for one invocation under one output-byte budget. It may contain native output sequence bytes, bypass bytes produced by raw output or proxy-on-error behavior, or both.
_Avoid_: Native output sequence, output stream

**Row document**:
A document in a row-framed native format whose root value is an object. The sequence header supplies the object keys, and the row supplies their values.
_Avoid_: Record, table document

**Document format**:
The syntax and data model used to represent one document, independent of how neighboring documents are separated.
_Avoid_: Native format, framing

**Framing**:
The rules that identify or emit any sequence header and the boundaries of ordered frames. Framing may inspect syntax needed to find a boundary, but it does not map document contents to values.
_Avoid_: Document format, serialization

**Frame detection**:
Identifying a frame's byte boundaries from framing rules independently of semantic document decoding. It may proceed alongside decoding, but tq cannot safely continue when malformed framing leaves the next boundary ambiguous.
_Avoid_: Record splitting, error recovery

**Frame**:
A byte segment identified by framing as exactly one candidate document. A frame still exists when its contents fail to decode into a document.
_Avoid_: Record, message

**Recovery boundary**:
A byte position where the selected input profile permits decoding to resume after a document failure. A record separator provides an explicit recovery boundary; jq-compatible profiles may also permit a decoder reset within the same recovery segment.
_Avoid_: Document boundary, frame boundary

**Recovery segment**:
The bytes from one record separator to the next or EOF in JSON sequence input. A segment may contain multiple document frames for jq compatibility, or no complete document.
_Avoid_: Frame, document

**Recoverable document failure**:
A document decoding failure after which the selected input profile permits further decoding without retracting prior observations. Recovery capability is separate from caller policy: top-level sequence input may warn and continue, while input-sequence access may expose the failure to the query.
_Avoid_: Parse recovery, malformed frame

**Document decoder**:
The directional component that interprets bytes assigned to one frame under a document format, input profile, and sequence context. It produces the document's root value or structural events without deciding frame boundaries.
_Avoid_: Parser, frame decoder

**Document encoder**:
The directional component that represents one encodable root value under a document format, output profile, and sequence context. It produces document bytes without emitting sequence headers or frame boundaries.
_Avoid_: Serializer, frame writer

**Sequence context**:
Format information established once for a document sequence and applied to each frame without becoming a document or value. A CSV or TSV row shape is sequence context.
_Avoid_: Parser state, global metadata

**Sequence header**:
The source representation that establishes sequence context without becoming a document. CSV and TSV headers establish the row shape used by each row document.
_Avoid_: Header document, first record

**Sequence preamble**:
Bytes before the first framing delimiter in an explicitly selected framed input. RFC 7464 mode discards its preamble for jq compatibility, but content probing recognizes the format only when record separator is the first non-whitespace byte.
_Avoid_: Leading junk, first frame

**Row shape**:
The ordered set of unique object keys declared by an input sequence header or the first output row document. A row document may omit those keys or assign them null, but it may not add keys outside the row shape.
_Avoid_: Table schema, inferred columns

**Row width**:
The number of fields in a row relative to its row shape. Missing trailing fields decode as null, while excess fields are a profile rejection because they have no declared keys.
_Avoid_: Record length, column count

**Optional field**:
A row-shape field that may be absent or null in a row document. CSV and TSV represent both as an unquoted empty field and decode that field as null; a quoted empty field represents an empty string.
_Avoid_: Empty string, missing column

**Scalar field**:
A row-document field whose value is a string, number, boolean, or null; arrays and objects are rejected. CSV and TSV decode quoted fields as strings, unquoted JSON booleans and numbers by type, unquoted empty fields as null, and other unquoted fields as strings.
_Avoid_: Cell object, nested field

**Type-preserving quoting**:
The CSV and TSV output rule that quotes a string when its unquoted text would decode as a number, boolean, or null. Numbers and booleans remain unquoted, while empty strings are quoted and nulls are unquoted empty fields.
_Avoid_: Minimal quoting, quote-all strings

**XML node profile**:
The default XML profile, representing XML as explicit ordered node values rather than projecting element names into object keys. It preserves node kinds, names, namespaces, attributes, and child order needed for semantic round trips.
_Avoid_: XML object mapping, lossless JSON

**XML document node**:
The root value of every document under the XML node profile. Its ordered children contain the root element and any retained document-level comments or processing instructions.
_Avoid_: Root element, wrapper object

**Expanded name**:
An XML element or attribute name represented by its namespace URI, or null when absent, and its local name. Source prefixes are syntax metadata and need not survive a semantic round trip.
_Avoid_: Qualified name, prefixed name

**XML element node**:
An XML node value with an expanded name, an array of expanded-name and string-value attributes, and an ordered child-node array. Namespace declarations resolve names but are not ordinary attributes.
_Avoid_: Element object, tag map

**XML content node**:
An element, text, comment, or processing-instruction value in the XML node profile. CDATA sections and character-reference spelling normalize into text nodes.
_Avoid_: XML token, mixed-content field

**Self-contained XML input**:
An XML document whose meaning can be decoded from its frame alone. tq rejects document type declarations and never resolves external entities, external DTDs, schemas, or XInclude; built-in entities and numeric character references normalize into text.
_Avoid_: Safe XML, DTD support

**XML object projection**:
An opt-in XML normalization that maps element names to object keys and repeated elements to arrays, with reserved keys for attributes and text. It favors concise queries over semantic round trips where XML distinctions collapse.
_Avoid_: Default XML profile, automatic XML mapping

**TOML temporal normalization**:
The TOML input rule that maps offset date-time, local date-time, local date, and local time values to strings. It discards the source distinction between TOML temporal values and strings.
_Avoid_: Date coercion, datetime preservation

**TOML number profile**:
The TOML mapping that preserves finite integers and floats as numbers within tq's numeric limits. It rejects `inf` and `nan` because the value model has no non-finite numbers.
_Avoid_: 64-bit number profile, special-float normalization

**TOML root table**:
The object root value of a TOML document. An encodable TOML root contains no null at any depth; the output profile also rejects scalar and array roots rather than placing them under an invented key.
_Avoid_: Root value wrapper, top-level TOML value

**INI section object**:
The object value established by one INI section beneath the document's root object; every section name, including `DEFAULT`, is literal and has no inheritance. Unsectioned keys remain direct root members, section-name punctuation stays literal unless key expansion is selected, and scalar/section collisions are key-shape conflicts.
_Avoid_: Section prefix, flat section

**String-map profile**:
The default INI and Properties mapping where every assigned source value becomes an uninterpolated string, including empty values and text that resembles a boolean or number. Output uses scalar stringification; producing typed scalars on input requires an explicit scalar-inference normalization.
_Avoid_: Typed configuration, automatic scalar parsing

**Scalar stringification**:
The default output normalization for string-map formats that writes strings unchanged and renders finite numbers, booleans, and null as canonical text. It rejects arrays and objects in leaf positions.
_Avoid_: Scalar coercion, JSON encoding

**Native format**:
A selectable tq input or output mode formed from a document format, framing, and format profile.
_Avoid_: Format string, document format

**Probeable format**:
A native input format that bounded source bytes can distinguish without guessing, such as XML or an RFC 7464 JSON sequence. Other formats require explicit selection or recognized source metadata.
_Avoid_: Guessable format, fallback parser

**Metadata-selected format**:
A native input format whose syntax cannot be distinguished by a bounded probe and therefore requires explicit selection or recognized source metadata. CSV, TSV, TOML, INI, and Properties are metadata-selected formats.
_Avoid_: Non-probeable format, extension-only format

**Format commitment**:
The point where a selected native input format becomes final. A later decoding failure belongs to that format and does not trigger another format candidate.
_Avoid_: Output commitment, fallback

**Output commitment**:
The point where encoded bytes for one output document become externally visible and cannot be withdrawn. Output-profile and sequence-context validation precede commitment, but an I/O or resource failure may leave partial bytes afterward.
_Avoid_: Format commitment, flush

**Format profile**:
The declared directional mapping between a format's data model and tq values. It may discard syntax metadata, but it must explicitly map or reject meaning that a value cannot represent.
_Avoid_: Parser behavior, conversion rules

**Format normalization**:
A declared profile rule that maps distinct source meanings to the same value or output representation. Normalized cases are supported but fall outside semantic round-trip guarantees.
_Avoid_: Coercion, lossy conversion

**Last-value-wins normalization**:
An opt-in input normalization that resolves duplicate keys within one object scope by retaining only the final value at its final occurrence's position. Without it, a duplicate key is a profile rejection.
_Avoid_: Last-write-wins, duplicate overwrite

**Declared structure**:
A profile rule where nested values come only from constructs that the document format defines as structural. Key punctuation remains literal unless an explicit key-expansion option is selected.
_Avoid_: Implicit nesting, automatic path expansion

**Key expansion**:
An explicit normalization that interprets dot-separated object-key segments as a nested object path within one document. Arrays remain arrays, while object keys inside their elements expand recursively; literal dotted keys require a different profile.
_Avoid_: Path inference, implicit expansion

**Key flattening**:
An explicit normalization that represents nested object paths as dot-joined keys within one document. Arrays remain arrays, while objects inside their elements flatten recursively; literal dotted keys are rejected.
_Avoid_: Stringification, flattened document

**Key-shape conflict**:
A profile rejection where key expansion or flattening would make one object path both a leaf value and an object, or would map multiple source paths to one key. Ordinary validation is scoped to one document.
_Avoid_: Mapping conflict, overwrite

**Profile rejection**:
A failure where the selected format profile cannot map an otherwise valid format construct or value. It is distinct from malformed framing, malformed document syntax, and query failure.
_Avoid_: Parse error, unsupported format

**Input profile**:
A format profile that defines how a document format's constructs and any sequence context become the root and nested values of each document.
_Avoid_: Parser, output profile

**Output profile**:
A format profile that defines how an encodable value becomes the root value of one document.
_Avoid_: Serializer, input profile

**Encodable value**:
A value accepted by an output profile for representation as one document. Encoding may normalize distinctions that the document format cannot carry.
_Avoid_: Supported value, compatible data

**Semantic round trip**:
An encodable value declared round-trippable by its output profile returns as an equal value through the corresponding input profile. This does not promise identical bytes or syntax metadata.
_Avoid_: Round trip, lossless round trip

**Profile round trip**:
An encodable value returns through the corresponding input profile as the value produced by the output profile's declared normalizations. Value distinctions may collapse, so a profile round trip does not imply a semantic round trip.
_Avoid_: Lossy round trip, semantic round trip

**Strict conversion**:
An opt-in conversion policy that rejects a result when the selected output profile requires a normalization that prevents a semantic round trip. Default conversion permits declared normalizations; strict rejection is atomic per document, while earlier frames may already be committed.
_Avoid_: Strict parsing, lossless mode

**Format string**:
A query construct beginning with `@` that converts values into string results, optionally through an interpolated template. A format string is not native format input or output.
_Avoid_: Formatter, output format

## Incremental processing

**Structural event**:
One ordered observation of input structure or scalar data that allows a query to run without retaining the complete value.
_Avoid_: Streaming value, token

**Event execution**:
An execution plan that consumes structural events without creating the complete input value. It preserves ordinary query inputs unless event input mode is selected.
_Avoid_: Stream plan, streaming query

**Event input mode**:
The jq-compatible input mode selected by `--stream`, where a query receives path/value and container-end values instead of a complete input value. It defines query semantics, not how tq decodes or encodes documents.
_Avoid_: Stream mode, streaming input

**Streaming decoder**:
A document decoder that emits structural events while consuming one frame without retaining the complete document.
_Avoid_: Chunked decoder, event input mode

**Streaming encoder**:
A document encoder that writes completed portions of one document without retaining the complete encoded result.
_Avoid_: Chunked encoder, formatter

**Bounded retention**:
A processing guarantee that retained data depends on active structure, deliberate query state, and configured limits rather than all completed input or output.
_Avoid_: Incremental processing, chunked processing

**Transcode**:
A conversion between native formats that connects a streaming decoder to a streaming encoder without creating the complete value. The input and output profiles must preserve identity-query semantics.
_Avoid_: Document conversion, event input mode
