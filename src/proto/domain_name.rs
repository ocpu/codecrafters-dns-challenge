use std::{fmt, hash::Hash};

use super::label::{Label, LabelError};

#[derive(Clone, Copy)]
pub struct DomainName<'data>(Option<Label<'data>>, usize);

impl<'data> DomainName<'data> {
    pub fn size_in_packet(&self) -> usize {
        let mut len = 0;
        if let Some(start) = self.0 {
            for label in start {
                len += match label
                    .expect("Domain name labels to be validated before calling size_in_packet")
                {
                    Label::Data { data, .. } => 1 + data.len(),
                    Label::Pointer { .. } => return len + 2,
                }
            }
        }

        1 + len
    }

    pub fn len(&self) -> usize {
        self.1
    }

    pub fn iter(&self) -> impl Iterator<Item = &'data str> {
        // Unwrap is safe as we check all labels during parse
        self.0
            .into_iter()
            .flat_map(|l| l)
            .map(|l| l.unwrap())
            .filter_map(|l| l.data())
    }
}

impl<'data> super::FromPacketBytes<'data> for DomainName<'data> {
    type Error = LabelError;

    fn parse(bytes: &'data [u8], offset: usize) -> Result<Option<Self>, Self::Error> {
        let Some(label) = Label::parse(bytes, offset)? else {
            return Ok(Some(Self(None, 0)));
        };
        let mut len = 0;

        for res in label {
            let res = res?;
            if res.data().is_some() {
                len += 1;
            }
        }

        Ok(Some(Self(Some(label), len)))
    }
}

impl<'data> fmt::Display for DomainName<'data> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(start) = self.0 {
            for item in start {
                let item = item.map_err(|_| std::fmt::Error)?;
                if let Some(data) = item.data() {
                    write!(f, "{data}.")?;
                }
            }
        } else {
            write!(f, ".")?;
        }
        Ok(())
    }
}

impl<'data> fmt::Debug for DomainName<'data> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(start) = self.0 {
            write!(f, "\"")?;
            for item in start {
                let item = item.map_err(|_| std::fmt::Error)?;
                if let Some(data) = item.data() {
                    write!(f, "{data}.")?;
                }
            }
            write!(f, "\"")?;
        } else {
            write!(f, "<ROOT>")?;
        }
        Ok(())
    }
}

impl<'data> Hash for DomainName<'data> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if let Some(start) = self.0 {
            for label in start {
                label
                    .expect("Domain name labels to be validated before running hash.")
                    .hash(state);
            }
        }
    }
}
