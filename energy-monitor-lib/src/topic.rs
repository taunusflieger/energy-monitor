use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::{fmt, marker::PhantomData};

pub trait Encode {
    fn encode(message: &Self) -> Bytes;
}

pub trait Decode {
    type Output;
    type DecodeError;

    fn decode<T: AsRef<[u8]>>(payload: T) -> Result<Self::Output, Self::DecodeError>;
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct Topic<M>(&'static str, PhantomData<M>);

impl<M> Topic<M>
where
    M: Encode + Decode,
{
    pub const fn new(topic: &'static str) -> Self {
        Self(topic, PhantomData {})
    }

    pub fn encode(&self, message: &M) -> Bytes {
        M::encode(message)
    }

    pub fn decode<T>(&self, payload: T) -> Result<M::Output, M::DecodeError>
    where
        T: AsRef<[u8]>,
    {
        M::decode(payload)
    }

    pub const fn name(&self) -> &'static str {
        self.0
    }
}

impl<T> Encode for T
where
    T: Serialize,
{
    fn encode(message: &Self) -> Bytes {
        serde_json::to_string(message).unwrap().into()
    }
}

impl<M> Decode for M
where
    M: for<'a> Deserialize<'a>,
{
    type Output = Self;
    type DecodeError = serde_json::Error;

    fn decode<T: AsRef<[u8]>>(payload: T) -> Result<Self::Output, Self::DecodeError> {
        serde_json::from_slice(payload.as_ref())
    }
}

impl<T> fmt::Display for Topic<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
