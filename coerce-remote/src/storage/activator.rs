use crate::storage::state::{ActorStore, ActorStoreErr};
use coerce_rt::actor::{Actor, ActorId, ActorState, FromActorState};

pub struct ActorActivator {
    store: Box<dyn ActorStore + Sync + Send>,
}

impl ActorActivator {
    pub fn new(store: Box<dyn ActorStore + Sync + Send>) -> ActorActivator {
        ActorActivator { store }
    }
}

pub struct DefaultActorStore;

#[async_trait]
impl ActorStore for DefaultActorStore {
    async fn get(&mut self, actor_id: ActorId) -> Result<Option<ActorState>, ActorStoreErr> {
        Ok(None)
    }

    async fn put(&mut self, actor: &ActorState) -> Result<(), ActorStoreErr> {
        Ok(())
    }

    async fn remove(&mut self, actor_id: ActorId) -> Result<bool, ActorStoreErr> {
        Ok(false)
    }

    fn clone(&self) -> Box<dyn ActorStore + Send + Sync> {
        Box::new(DefaultActorStore)
    }
}

impl ActorActivator {
    pub async fn activate<A: Actor>(&mut self, id: &ActorId) -> Option<A>
    where
        A: FromActorState<A>,
    {
        let id = id.clone();
        let state = self.store.get(id).await;
        if let Ok(Some(state)) = state {
            A::try_from(state)
        } else {
            None
        }
    }
}

impl Clone for ActorActivator {
    fn clone(&self) -> Self {
        ActorActivator {
            store: self.store.clone(),
        }
    }
}
