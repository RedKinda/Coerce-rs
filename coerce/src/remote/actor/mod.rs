use crate::remote::cluster::node::RemoteNodeStore;

use crate::actor::message::Message;
use crate::actor::system::ActorSystem;
use crate::actor::{scheduler::ActorType, Actor, ActorId, LocalActorRef};
use crate::remote::handler::{
    ActorHandler, ActorMessageHandler, RemoteActorMarker, RemoteActorMessageMarker,
};
use crate::remote::net::client::RemoteClientStream;
use crate::remote::system::{NodeId, RemoteActorSystem};
use std::any::TypeId;
use std::collections::HashMap;

use crate::actor::scheduler::ActorType::Anonymous;
use crate::remote::stream::pubsub::Subscription;

use crate::remote::cluster::sharding::host::RemoteEntityRequest;
use crate::remote::heartbeat::HeartbeatConfig;
use uuid::Uuid;

pub mod handler;
pub mod message;

pub struct RemoteClientRegistry {
    clients: HashMap<NodeId, Box<dyn RemoteClientStream + Sync + Send>>,
}

pub struct RemoteRegistry {
    nodes: RemoteNodeStore,
    actors: HashMap<ActorId, NodeId>,
    system: Option<RemoteActorSystem>,
    system_event_subscription: Option<Subscription>,
}

pub(crate) type BoxedActorHandler = Box<dyn ActorHandler + Send + Sync>;

pub(crate) type BoxedMessageHandler = Box<dyn ActorMessageHandler + Send + Sync>;

pub struct RemoteSystemConfig {
    node_tag: String,
    actor_types: HashMap<TypeId, String>,
    handler_types: HashMap<TypeId, String>,
    message_handlers: HashMap<String, BoxedMessageHandler>,
    actor_handlers: HashMap<String, BoxedActorHandler>,
    heartbeat_config: HeartbeatConfig,
}

impl RemoteSystemConfig {
    pub fn new(
        node_tag: String,
        actor_types: HashMap<TypeId, String>,
        handler_types: HashMap<TypeId, String>,
        message_handlers: HashMap<String, BoxedMessageHandler>,
        actor_handlers: HashMap<String, BoxedActorHandler>,
    ) -> RemoteSystemConfig {
        RemoteSystemConfig {
            node_tag,
            actor_types,
            handler_types,
            message_handlers,
            actor_handlers,
            heartbeat_config: HeartbeatConfig::default(),
        }
    }

    pub fn node_tag(&self) -> &str {
        &self.node_tag
    }

    pub fn handler_name<A: Actor, M: Message>(
        &self,
        marker: RemoteActorMessageMarker<A, M>,
    ) -> Option<String> {
        self.handler_types
            .get(&marker.id())
            .map(|name| name.clone())
    }

    pub fn actor_name<A: Actor>(&self, marker: RemoteActorMarker<A>) -> Option<String>
    where
        A: 'static + Send + Sync,
    {
        self.actor_types.get(&marker.id()).map(|name| name.clone())
    }

    pub fn message_handler(&self, key: &String) -> Option<BoxedMessageHandler> {
        self.message_handlers
            .get(key)
            .map(|handler| handler.new_boxed())
    }

    pub fn actor_handler(&self, key: &String) -> Option<BoxedActorHandler> {
        self.actor_handlers
            .get(key)
            .map(|handler| handler.new_boxed())
    }
}

pub struct RemoteHandler {
    requests: HashMap<Uuid, RemoteRequest>,
}

impl RemoteHandler {
    pub fn push_request(&mut self, message_id: Uuid, request: RemoteRequest) {
        self.requests.insert(message_id, request);
    }

    pub fn pop_request(&mut self, message_id: Uuid) -> Option<RemoteRequest> {
        self.requests.remove(&message_id)
    }
}

pub struct RemoteRequest {
    pub res_tx: tokio::sync::oneshot::Sender<RemoteResponse>,
}

#[derive(Debug)]
pub enum RemoteResponse {
    Ok(Vec<u8>),
    Err(Vec<u8>),
}

impl RemoteResponse {
    pub fn is_ok(&self) -> bool {
        match self {
            &RemoteResponse::Ok(..) => true,
            _ => false,
        }
    }

    pub fn is_err(&self) -> bool {
        match self {
            &RemoteResponse::Err(..) => true,
            _ => false,
        }
    }

    pub fn into_result(self) -> Result<Vec<u8>, Vec<u8>> {
        match self {
            RemoteResponse::Ok(buff) => Ok(buff),
            RemoteResponse::Err(buff) => Err(buff),
            _ => panic!("response is not a buffer"),
        }
    }
}

impl Actor for RemoteRegistry {}

impl Actor for RemoteHandler {}

impl Actor for RemoteClientRegistry {}

impl RemoteClientRegistry {
    pub async fn new(ctx: &ActorSystem, system_tag: &str) -> LocalActorRef<RemoteClientRegistry> {
        ctx.new_actor(
            format!("RemoteClientRegistry-{}", system_tag),
            RemoteClientRegistry {
                clients: HashMap::new(),
            },
            ActorType::Anonymous,
        )
        .await
        .expect("RemoteClientRegistry")
    }

    pub fn add_client<C: RemoteClientStream>(&mut self, node_id: NodeId, client: C)
    where
        C: 'static + Sync + Send,
    {
        self.clients.insert(node_id, Box::new(client));
    }

    pub fn remove_client(&mut self, node_id: NodeId) {
        self.clients.remove(&node_id);
    }
}

impl RemoteRegistry {
    pub async fn new(ctx: &ActorSystem, system_tag: &str) -> LocalActorRef<RemoteRegistry> {
        ctx.new_actor(
            format!("RemoteRegistry-{}", &system_tag),
            RemoteRegistry {
                actors: HashMap::new(),
                nodes: RemoteNodeStore::new(vec![]),
                system: None,
                system_event_subscription: None,
            },
            Anonymous,
        )
        .await
        .expect("RemoteRegistry")
    }
}

impl RemoteHandler {
    pub fn new() -> RemoteHandler {
        RemoteHandler {
            requests: HashMap::new(),
        }
    }
}
