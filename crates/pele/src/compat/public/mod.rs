//! Mutable host documents augment the immutable typed Rust API.
//! Public codecs are opt-in; an unimplemented codec never silently ignores edits.
mod engine;
mod graph;
mod schema;
pub use engine::{PublicDiagramModule, PublicDocument, PublicEngine};
pub use graph::{Atom, GraphWriter, JsString, ModelGraph, Node, Property};

pub use schema::{
    FieldKind, FieldSchema, FunctionShape, HostExceptionToken, InvocationOutcome,
    JavaScriptErrorKind, MemberKind, MemberSchema, MethodCall, MethodId, MethodSchema, ModelSchema,
    ModelSchemaView, PrototypeId, PrototypeParent, PrototypeSchema, PublicError, PublicPeleError,
    RecordId, RecordKind, RecordSchema, RenderOutcome, SharedFunction, OPAQUE_MODEL_SCHEMA,
};

mod binary;
pub use binary::{
    decode_binary_exchange, encode_binary_exchange, BinaryExchange, DEFAULT_BINARY_BUDGET,
};
