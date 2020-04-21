use std::path::Path;
use std::{fmt, fs};

use mqtt_broker::auth::{Activity, Authorizer, MakeAuthorizer};
use opa_go::Rego;

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

pub struct OpaAuthorizer {
    rego: Rego,
}

impl OpaAuthorizer {
    pub fn from_rego<P: AsRef<Path>>(query: &str, path: P) -> Result<Self, Error> {
        let name = path.as_ref().file_name().unwrap().to_str().unwrap();
        let contents = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        let rego = Rego::new(query, name, &contents).unwrap();
        let auth = Self { rego };
        Ok(auth)
    }
}

impl MakeAuthorizer for OpaAuthorizer {
    type Authorizer = OpaAuthorizer;
    type Error = Error;

    fn make_authorizer(self) -> Result<Self::Authorizer, Self::Error> {
        Ok(self)
    }
}

impl Authorizer for OpaAuthorizer {
    type Error = Error;

    fn authorize(&mut self, activity: Activity) -> Result<bool, Self::Error> {
        let result = self.rego.eval_bool(&activity).unwrap();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
