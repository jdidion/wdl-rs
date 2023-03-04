macro_rules! name_const {
    ( $( $c:ident ), * ) => {
        $(
        paste::paste! {
        pub const [<$c:upper>]: &str = stringify!($c);
        }
        )*
    };
}

macro_rules! name_value_const {
    ( $( $c:ident => $s:literal), * ) => {
        $(
        paste::paste! {
        pub const [<$c:upper>]: &str = $s;
        }
        )*
    };
}

pub mod rules {
    // Constants for sytanx elements (node and field names)
    name_const!(
        and_operator,
        apply_expression,
        array,
        array_type,
        binary_operator,
        boolean_type,
        bound_declaration,
        call,
        command,
        comment,
        comparison_operator,
        conditional,
        content,
        dec_int,
        escape_sequence,
        false,
        field_expression,
        file_type,
        float,
        float_type,
        group_expression,
        hex_int,
        identifier,
        import,
        index_expression,
        input,
        int_type,
        map,
        map_type,
        meta,
        meta_array,
        meta_object,
        none,
        not_operator,
        null,
        object,
        object_type,
        oct_int,
        optional_type,
        or_operator,
        output,
        pair,
        pair_type,
        parameter_meta,
        placeholder,
        runtime,
        scatter,
        simple_string,
        string,
        string_type,
        struct,
        task,
        ternary_expression,
        true,
        unary_operator,
        unbound_declaration,
        user_type,
        workflow
    );
}

pub mod fields {
    name_const!(
        alias,
        aliases,
        arguments,
        attributes,
        body,
        collection,
        condition,
        declarations,
        elements,
        entries,
        expression,
        false,
        fields,
        from,
        index,
        inputs,
        left,
        identifier,
        key,
        name,
        namespace,
        nonempty,
        object,
        operator,
        parts,
        right,
        target,
        to,
        true,
        type,
        uri,
        value,
        version
    );
}

pub mod keywords {
    name_const!(
        as,
        alias,
        call,
        command,
        else,
        if,
        import,
        in,
        input,
        meta,
        output,
        parameter_meta,
        runtime,
        scatter,
        struct,
        task,
        then,
        version,
        workflow
    );
    name_value_const!(
        array => "Array",
        map => "Map",
        pair => "Pair"
    );
}

pub mod symbols {
    name_value_const!(
        assign => "=",
        colon => ":",
        comma => ",",
        dot => ".",
        lbrace => "{",
        lbrack => "[",
        lparen => "(",
        optional => "?",
        rbrace => "}",
        rbrack => "]",
        rparen => ")",
        squote => "'",
        dquote => "\""
    );
}
