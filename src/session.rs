use crate::utils::id;
use futures::lock::Mutex;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone)]
pub struct Session {
    pub key: Uuid,
    pub user_id: Uuid,
    pub csrf_token: Uuid,
}

impl Session {
    pub fn new(user_id: &Uuid) -> Session {
        Session {
            key: id(),
            user_id: user_id.clone(),
            csrf_token: Uuid::new_v4(),
        }
    }
}

pub struct SessionMap {
    inner: Mutex<HashMap<Uuid, Session>>,
}

static SESSION_MAP: OnceCell<SessionMap> = OnceCell::new();

impl SessionMap {
    pub fn new() -> SessionMap {
        SessionMap {
            inner: Mutex::new(HashMap::new()),
        }
    }

    pub async fn start(&self, user_id: &Uuid) -> Session {
        let mut inner = self.inner.lock().await;
        let session = Session::new(user_id);
        let key = session.key.clone();
        inner.insert(key, session.clone());
        session
    }

    pub async fn get_session(&self, key: &Uuid) -> Option<Session> {
        let inner = self.inner.lock().await;
        inner.get(key).map(Clone::clone)
    }

    pub fn get() -> &'static SessionMap {
        SESSION_MAP.get_or_init(SessionMap::new)
    }
}
