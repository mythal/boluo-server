table! {
    channel_members (user_id, channel_id) {
        user_id -> Uuid,
        channel_id -> Uuid,
        join_date -> Timestamp,
        character_name -> Text,
    }
}

table! {
    channels (id) {
        id -> Uuid,
        name -> Text,
        topic -> Text,
        space_id -> Uuid,
        created -> Timestamp,
        is_public -> Bool,
        deleted -> Bool,
    }
}

table! {
    media (id) {
        id -> Uuid,
        mine_type -> Text,
        uploader_id -> Uuid,
        filename -> Text,
        original_filename -> Text,
        hash -> Text,
        size -> Int4,
        description -> Text,
        created -> Timestamp,
    }
}

table! {
    messages (id) {
        id -> Uuid,
        sender_id -> Uuid,
        channel_id -> Uuid,
        parent_message_id -> Nullable<Uuid>,
        name -> Text,
        media_id -> Nullable<Uuid>,
        seed -> Bytea,
        deleted -> Bool,
        in_game -> Bool,
        is_action -> Bool,
        is_master -> Bool,
        pinned -> Bool,
        tags -> Array<Text>,
        folded -> Bool,
        text -> Text,
        whisper_to_users -> Nullable<Array<Uuid>>,
        entities -> Jsonb,
        created -> Timestamp,
        modified -> Timestamp,
        order_date -> Timestamp,
        order_offset -> Int4,
    }
}

table! {
    restrained_members (user_id, space_id) {
        user_id -> Uuid,
        space_id -> Uuid,
        blocked -> Bool,
        muted -> Bool,
        restrained_date -> Timestamp,
        operator_id -> Nullable<Uuid>,
    }
}

table! {
    space_members (user_id, space_id) {
        user_id -> Uuid,
        space_id -> Uuid,
        is_master -> Bool,
        is_admin -> Bool,
        join_date -> Timestamp,
    }
}

table! {
    spaces (id) {
        id -> Uuid,
        name -> Text,
        description -> Text,
        created -> Timestamp,
        modified -> Timestamp,
        owner_id -> Uuid,
        is_public -> Bool,
        deleted -> Bool,
        password -> Text,
        language -> Text,
        default_dice_type -> Text,
    }
}

table! {
    users (id) {
        id -> Uuid,
        email -> Text,
        username -> Text,
        nickname -> Text,
        password -> Text,
        bio -> Text,
        joined -> Timestamp,
        deactivated -> Bool,
        avatar_id -> Nullable<Uuid>,
    }
}

joinable!(channel_members -> channels (channel_id));
joinable!(channel_members -> users (user_id));
joinable!(channels -> spaces (space_id));
joinable!(messages -> channels (channel_id));
joinable!(messages -> users (sender_id));
joinable!(restrained_members -> spaces (space_id));
joinable!(space_members -> spaces (space_id));
joinable!(space_members -> users (user_id));
joinable!(spaces -> users (owner_id));

allow_tables_to_appear_in_same_query!(
    channel_members,
    channels,
    media,
    messages,
    restrained_members,
    space_members,
    spaces,
    users,
);
