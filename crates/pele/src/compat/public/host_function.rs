//! Borrowed, bounded access to an active model arena. IDs retain arena identity;
//! implementations update their native indexes rather than copying a model graph.
use super::{Atom, JsString, MethodCall, PublicError};
use crate::compat::RenderResources;

pub trait ActiveModel {
    fn root(&self) -> Atom;
    fn validate(&self, value: &Atom) -> Result<(), PublicError>;
    fn read(&mut self, receiver: &Atom, key: &str) -> Result<Atom, PublicError>;
    fn set(&mut self, receiver: &Atom, key: &str, value: Atom) -> Result<(), PublicError>;
    fn intern(&mut self, value: JsString) -> Result<Atom, PublicError>;
    fn string(&mut self, value: &Atom) -> Result<JsString, PublicError>;
    fn object(&mut self) -> Result<Atom, PublicError>;
    fn array(&mut self) -> Result<Atom, PublicError>;
    fn date(&mut self, milliseconds: f64) -> Result<Atom, PublicError>;
    fn date_millis(&mut self, value: &Atom) -> Result<f64, PublicError>;
    fn push(&mut self, array: &Atom, value: Atom) -> Result<(), PublicError>;
    /// Dispatch the captured native method, never a replaced own property.
    fn invoke_builtin(
        &mut self,
        call: MethodCall,
        resources: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError>;
}

/// An owner-retained callable. No Send/Sync promise; call and model borrows are
/// synchronous. Resolving the owned callable before invocation lets the original
/// built-in use the same render session without aliasing the host adapter.
pub trait HostFunction {
    fn call(
        &self,
        model: &mut dyn ActiveModel,
        receiver: Atom,
        arguments: &[Atom],
        resources: &mut dyn RenderResources,
    ) -> Result<Atom, PublicError>;
}
