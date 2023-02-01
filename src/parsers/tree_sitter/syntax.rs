use paste::paste;

macro_rules! name_const {
    ( $( $c:ident ), * ) => {
        $(
        paste! {
        pub const [<$c:upper>]: &str = stringify!($c);
        }
        )*
    };
}

// Constants for sytanx elements (node and field names)
name_const!(
    alias,
    aliases,
    and_operator,
    apply_expression,
    arguments,
    array,
    array_type,
    attributes,
    binary_operator,
    body,
    boolean_type,
    bound_declaration,
    call,
    collection,
    command,
    comment,
    comparison_operator,
    condition,
    conditional,
    content,
    dec_int,
    declarations,
    elements,
    entries,
    escape_sequence,
    expression,
    false,
    field_expression,
    fields,
    file_type,
    float,
    float_type,
    from,
    group_expression,
    hex_int,
    identifier,
    import,
    index,
    index_expression,
    input,
    inputs,
    int_type,
    key,
    left,
    map,
    map_type,
    meta,
    meta_array,
    meta_object,
    name,
    namespace,
    none,
    nonempty_array_type,
    not_operator,
    null,
    object,
    object_type,
    oct_int,
    operator,
    optional_type,
    or_operator,
    output,
    pair,
    pair_type,
    parameter_meta,
    parts,
    placeholder,
    right,
    runtime,
    scatter,
    simple_string,
    string,
    string_type,
    struct,
    target,
    task,
    ternary_expression,
    to,
    true,
    type,
    unary_operator,
    unbound_declaration,
    uri,
    user_type,
    value,
    version,
    workflow
);
