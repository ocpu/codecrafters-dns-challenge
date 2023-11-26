use crate::{
    domain_name::DomainName, proto::{QType, QClass},
};

#[derive(Debug, Clone)]
pub struct Question {
    name: DomainName,
    q_type: QType,
    q_class: QClass,
}

impl Question {
    pub fn new(q_type: QType, q_class: QClass, name: DomainName) -> Self {
        Self {
            name,
            q_type,
            q_class,
        }
    }

    pub fn name(&self) -> &DomainName {
        &self.name
    }

    pub fn q_type(&self) -> &QType {
        &self.q_type
    }

    pub fn q_class(&self) -> &QClass {
        &self.q_class
    }
}

impl<'a> From<crate::proto::Question<'a>> for Question {
    fn from(value: crate::proto::Question) -> Self {
        Self::new(value.q_type(), value.q_class(), (&value.name()).into())
    }
}
