//! Mutable host documents augment the immutable typed Rust API.
//! Public codecs are opt-in; an unimplemented codec never silently ignores edits.
mod engine;
mod graph;
mod schema;
pub use engine::{PublicDiagramModule, PublicDocument, PublicEngine};
pub use graph::{Atom, GraphWriter, JsString, ModelGraph, Node, Property};

pub use schema::{
    FieldKind, FieldSchema, HostExceptionToken, InvocationOutcome, JavaScriptErrorKind, MemberKind,
    MemberSchema, MethodCall, MethodId, MethodSchema, ModelSchema, PrototypeId, PrototypeParent,
    PrototypeSchema, PublicError, PublicPeleError, RecordId, RecordKind, RecordSchema,
    OPAQUE_MODEL_SCHEMA,
};
