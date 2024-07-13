use std::{fmt::Display, sync::Arc};

#[derive(Debug, PartialEq)]
pub struct Nickname {
    pub value: Arc<str>,
}

impl Nickname {
    pub fn from_owned(value: String) -> Self {
        Self {
            value: value.into(),
        }
    }

    pub fn from_slice(value: &str) -> Self {
        Self {
            value: value.into(),
        }
    }

    pub fn as_slice(&self) -> &str {
        &self.value
    }
}

// Use explicit implementation of Clone instead of derived one for code clarity
impl Clone for Nickname {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
        }
    }
}

impl Display for Nickname {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.value)
    }
}

impl From<String> for Nickname {
    fn from(value: String) -> Self {
        Nickname::from_owned(value)
    }
}

impl From<&str> for Nickname {
    fn from(value: &str) -> Self {
        Nickname::from_slice(value)
    }
}

impl PartialEq<Nickname> for String {
    fn eq(&self, other: &Nickname) -> bool {
        other.as_slice().eq(self)
    }
}
