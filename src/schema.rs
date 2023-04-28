// @generated automatically by Diesel CLI.

diesel::table! {
    busses (placeid) {
        placeid -> Text,
        busid -> Text,
    }
}

diesel::table! {
    current_location (busid) {
        busid -> Text,
        latitude -> Float,
        longitude -> Float,
    }
}

diesel::table! {
    place_location (busid) {
        busid -> Text,
        latitude -> Float,
        longitude -> Float,
    }
}

diesel::table! {
    routes (busid) {
        busid -> Text,
        placeid -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    busses,
    current_location,
    place_location,
    routes,
);
