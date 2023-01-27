mod logger;
mod lookup;
mod serializer;
mod single;

use std::collections::HashMap;
use std::marker::PhantomData;

pub trait Db {}

pub struct SampleSchemaV1 {}

impl Db for SampleSchemaV1 {}
