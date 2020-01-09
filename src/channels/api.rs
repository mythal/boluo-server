use uuid::Uuid;

pub struct CreateSpace {
    name: String,
    owner_id: Uuid,
}
