use std::sync::Arc;

use evmap_derive::ShallowCopy;
use tokio::sync::mpsc;

use crate::{domain_name::DomainName, proto::{Type, QType}, resource::ResourceData};


pub fn new() -> (EVCache, EVCacheOperator) {
    let (thr, thw) = evmap::new();
    let (dnir, dniw) = evmap::new();
    let (dnatir, dnatiw) = evmap::new();
    let (ccs, ccr) = mpsc::channel(200);

    let cache = EVCache {
        table_handle: thr,
        domain_name_index: dnir,
        domain_name_and_type_index: dnatir,
        control_channel: ccs,
    };

    let operator = EVCacheOperator {
        table_handle: thw,
        domain_name_index: dniw,
        domain_name_and_type_index: dnatiw,
        control_channel: ccr,
    };

    (cache, operator)
}


#[derive(Debug, ShallowCopy, Clone, Hash, PartialEq, Eq)]
struct CacheKey(Arc<(DomainName, Type, Arc<[u8]>)>);

#[derive(Clone)]
pub struct EVCache {
    table_handle: evmap::ReadHandle<CacheKey, Arc<ResourceData>>,
    domain_name_index: evmap::ReadHandle<DomainName, CacheKey>,
    domain_name_and_type_index: evmap::ReadHandle<(DomainName, Type), CacheKey>,
    control_channel: mpsc::Sender<EVControlMessage>,
}

// NOTE: Knowing fully well that this comment might be outdated, this structure is Sync. Every
// operation on the structure produces new clones of the evmap::ReadHandle. In cases where data
// is extracted from the structure they are in a concurrenct safe container such as Arc or
// themselfs implemented as Arc-like.
unsafe impl Sync for EVCache {
}

impl EVCache {
    pub fn get(&self, key: impl Into<GetKey>) -> Option<Box<[Arc<ResourceData>]>> {
        let key = key.into();
        let keys = if let Some(typ) = key.1 {
            self.domain_name_and_type_index.get(&(key.0, typ))
        } else {
            self.domain_name_index.get(&key.0)
        };
        let Some(keys) = keys else {
            return None;
        };

        Some(keys.iter().filter_map(|key| self.table_handle.get_one(key).map(|v| Arc::clone(v.as_ref()))).collect::<Vec<_>>().into_boxed_slice())
    }

    pub fn bulk(&self) -> EVCacheBulk {
        EVCacheBulk {
            control_channel: self.control_channel.clone(),
        }
    }
}

pub struct EVCacheBulk {
    control_channel: mpsc::Sender<EVControlMessage>,
}

#[derive(Debug)]
pub struct CacheOperatorGone;

impl EVCacheBulk {
    pub async fn insert(
        self,
        domain_name: &DomainName,
        data: ResourceData,
    ) -> Result<Self, CacheOperatorGone> {
        self.control_channel
            .send(EVControlMessage::Insert(domain_name.clone(), data))
            .await
            .map_err(|_| CacheOperatorGone)?;
        Ok(self)
    }

    pub async fn publish(self) -> Result<(), CacheOperatorGone> {
        self.control_channel
            .send(EVControlMessage::Publish)
            .await
            .map_err(|_| CacheOperatorGone)?;
        Ok(())
    }
}

#[derive(Debug)]
enum EVControlMessage {
    Insert(DomainName, ResourceData),
    Publish,
}

pub struct EVCacheOperator {
    table_handle: evmap::WriteHandle<CacheKey, Arc<ResourceData>>,
    domain_name_index: evmap::WriteHandle<DomainName, CacheKey>,
    domain_name_and_type_index: evmap::WriteHandle<(DomainName, Type), CacheKey>,
    control_channel: mpsc::Receiver<EVControlMessage>,
}

impl EVCacheOperator {
    pub async fn listen(mut self) {
        while let Some(msg) = self.control_channel.recv().await {
            tracing::debug!("Received cache control message: {msg:?}");
            match msg {
                EVControlMessage::Insert(name, data) => {
                    let key = CacheKey(Arc::new((name.clone(), *data.typ(), Arc::from(data.data().as_ref()))));
                    self.domain_name_and_type_index.insert((name.clone(), *data.typ()), key.clone());
                    self.domain_name_index.insert(name.clone(), key.clone());
                    self.table_handle.update(key, Arc::new(data));
                }
                EVControlMessage::Publish => {
                    self.table_handle.refresh();
                    self.domain_name_index.refresh();
                    self.domain_name_and_type_index.refresh();
                }
            }
        }
    }
}

pub struct GetKey(DomainName, Option<Type>);

macro_rules! convert_into_get_key {
    ($typ:ty: $id:pat_param => $expr:expr) => {
        impl From<$typ> for GetKey {
            fn from($id: $typ) -> Self {
                $expr
            }
        }
    };
}

fn into_type(typ: &QType) -> Option<Type> {
    match typ {
        QType::ALL => None,
        typ => Some(Type::from(typ.as_u16())),
    }
}

convert_into_get_key!(DomainName: dn => GetKey(dn, None));
convert_into_get_key!(&DomainName: dn => GetKey(dn.clone(), None));
convert_into_get_key!((DomainName, Type): (dn, typ) => GetKey(dn, Some(typ)));
convert_into_get_key!((&DomainName, &Type): (dn, typ) => GetKey(dn.clone(), Some(*typ)));
convert_into_get_key!((&DomainName, Type): (dn, typ) => GetKey(dn.clone(), Some(typ)));
convert_into_get_key!((DomainName, &Type): (dn, typ) => GetKey(dn, Some(*typ)));
convert_into_get_key!((DomainName, Option<Type>): (dn, typ) => GetKey(dn, typ));
convert_into_get_key!((&DomainName, Option<&Type>): (dn, typ) => GetKey(dn.clone(), typ.cloned()));
convert_into_get_key!((&DomainName, Option<Type>): (dn, typ) => GetKey(dn.clone(), typ));
convert_into_get_key!((DomainName, Option<&Type>): (dn, typ) => GetKey(dn, typ.cloned()));
convert_into_get_key!((DomainName, QType): (dn, typ) => GetKey(dn, into_type(&typ)));
convert_into_get_key!((&DomainName, &QType): (dn, typ) => GetKey(dn.clone(), into_type(typ)));
convert_into_get_key!((&DomainName, QType): (dn, typ) => GetKey(dn.clone(), into_type(&typ)));
convert_into_get_key!((DomainName, &QType): (dn, typ) => GetKey(dn, into_type(typ)));
convert_into_get_key!((DomainName, Option<QType>): (dn, typ) => GetKey(dn, typ.as_ref().and_then(into_type)));
convert_into_get_key!((&DomainName, Option<&QType>): (dn, typ) => GetKey(dn.clone(), typ.and_then(into_type)));
convert_into_get_key!((&DomainName, Option<QType>): (dn, typ) => GetKey(dn.clone(), typ.as_ref().and_then(into_type)));
convert_into_get_key!((DomainName, Option<&QType>): (dn, typ) => GetKey(dn, typ.and_then(into_type)));
