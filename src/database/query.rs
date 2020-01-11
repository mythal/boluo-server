use postgres_types::Type;

#[derive(Copy, Clone)]
pub struct Query {
    pub key: Key,
    pub source: &'static str,
    pub types: &'static [Type],
}

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub struct Key(&'static str);

macro_rules! make {
    ($filename: expr, $types: expr) => {
        Query {
            key: Key($filename),
            source: include_str!(concat!("sql/", $filename, ".sql")),
            types: $types,
        }
    };
    ($filename: expr) => {
        make!($filename, &[])
    };
}

pub static ADD_USER_TO_CHANNEL: Query = make!("add_user_to_channel");
pub static ADD_USER_TO_SPACE: Query = make!("add_user_to_space");
pub static CREATE_CHANNEL: Query = make!("create_channel");
pub static CREATE_MESSAGE: Query = make!("create_message");
pub static CREATE_SPACE: Query = make!("create_space");
pub static CREATE_USER: Query = make!("create_user");
pub static DELETE_CHANNEL: Query = make!("delete_channel");
pub static DELETE_SPACE: Query = make!("delete_space");
pub static DELETE_USER: Query = make!("delete_user");
pub static EDIT_MESSAGE: Query = make!("edit_message");
pub static FETCH_CHANNEL: Query = make!("fetch_channel", &[Type::UUID]);
pub static FETCH_CHANNEL_MEMBER: Query = make!("fetch_channel_member");
pub static FETCH_MESSAGE: Query = make!("fetch_message");
pub static FETCH_SPACE: Query = make!("fetch_space", &[Type::UUID, Type::TEXT, Type::BOOL]);
pub static FETCH_SPACE_MEMBER: Query = make!("fetch_space_member");
pub static FETCH_USER: Query = make!("fetch_user", &[Type::UUID, Type::TEXT, Type::TEXT]);
pub static GET_MESSAGE_AND_SPACE_MEMBER: Query = make!("get_message_and_space_member");
pub static IS_SPACE_PUBLIC: Query = make!("is_space_public", &[Type::UUID, Type::TEXT, Type::TEXT]);
pub static LOGIN: Query = make!("login", &[Type::TEXT, Type::TEXT, Type::TEXT]);
pub static REMOVE_MESSAGE: Query = make!("remove_message", &[Type::UUID]);
pub static REMOVE_USER_FROM_ALL_CHANNEL_OF_THE_SPACE: Query =
    make!("remove_user_from_all_channel_of_the_space", &[Type::UUID]);
pub static REMOVE_USER_FROM_CHANNEL: Query = make!("remove_user_from_channel");
pub static REMOVE_USER_FROM_SPACE: Query = make!("remove_user_from_space");
pub static SELECT_CHANNEL_MEMBERS: Query = make!("select_channel_members");
pub static SELECT_MESSAGES: Query = make!("select_messages");
pub static SELECT_SPACES_CHANNELS: Query = make!("select_space_channels");
pub static SELECT_SPACE_MEMBERS: Query = make!("select_space_members");
pub static SELECT_SPACES: Query = make!("select_spaces");
pub static SELECT_USERS: Query = make!("select_users");
pub static SET_CHANNEL_MEMBER: Query = make!("set_channel_member");
pub static SET_SPACE_MEMBER: Query = make!("set_space_member");

pub static ALL_QUERY: &[Query] = &[
    ADD_USER_TO_CHANNEL,
    ADD_USER_TO_SPACE,
    CREATE_CHANNEL,
    CREATE_MESSAGE,
    CREATE_SPACE,
    CREATE_USER,
    DELETE_CHANNEL,
    DELETE_SPACE,
    DELETE_USER,
    EDIT_MESSAGE,
    FETCH_CHANNEL,
    FETCH_CHANNEL_MEMBER,
    FETCH_MESSAGE,
    FETCH_SPACE,
    FETCH_SPACE_MEMBER,
    FETCH_USER,
    GET_MESSAGE_AND_SPACE_MEMBER,
    IS_SPACE_PUBLIC,
    REMOVE_MESSAGE,
    REMOVE_USER_FROM_ALL_CHANNEL_OF_THE_SPACE,
    REMOVE_USER_FROM_CHANNEL,
    REMOVE_USER_FROM_SPACE,
    SELECT_CHANNEL_MEMBERS,
    SELECT_MESSAGES,
    SELECT_SPACES_CHANNELS,
    SELECT_SPACE_MEMBERS,
    SELECT_SPACES,
    SELECT_USERS,
    SET_CHANNEL_MEMBER,
    SET_SPACE_MEMBER,
    LOGIN,
];
