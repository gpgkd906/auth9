use crate::state::{HasDbPool, HasServices};

pub trait AuthorizationContext: HasServices + HasDbPool {}

impl<T> AuthorizationContext for T where T: HasServices + HasDbPool {}
