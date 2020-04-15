use std::fmt;
use std::path::Path;

use mqtt_broker::auth::{Activity, Authorizer, MakeAuthorizer};
use opa_wasm::Policy;

#[derive(Debug)]
pub enum Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error")
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

pub struct MakeOpaAuthorizer {
    module: Vec<u8>,
}

impl MakeOpaAuthorizer {
    pub fn from_rego<P: AsRef<Path>>(query: &str, path: P) -> Result<MakeOpaAuthorizer, Error> {
        let module = opa_compiler::compile(query, path).unwrap();
        let auth = Self { module };
        Ok(auth)
    }
}

impl MakeAuthorizer for MakeOpaAuthorizer {
    type Authorizer = OpaAuthorizer;
    type Error = Error;

    fn make_authorizer(self) -> Result<Self::Authorizer, Self::Error> {
        OpaAuthorizer::from_bytes(&self.module)
    }
}

pub struct OpaAuthorizer {
    policy: Policy,
}

impl OpaAuthorizer {
    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let policy = Policy::from_wasm(bytes).unwrap();
        let auth = Self { policy };
        Ok(auth)
    }
}

impl Authorizer for OpaAuthorizer {
    type Error = Error;

    fn authorize(&mut self, activity: Activity) -> Result<bool, Self::Error> {
        let value = self.policy.evaluate(&activity).unwrap();
        Ok(!value.try_into_set().unwrap().is_empty())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
