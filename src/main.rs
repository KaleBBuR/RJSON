use std::{collections::HashMap, fmt, iter::Peekable, str::Chars};
use serde_json::{Value, json};

fn main() {
    let data = r#"
        {
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ],
            "friends": [
                {
                    "first": "Dale",
                    "last": "Murphy",
                    "age": 44,
                    "nets": ["ig", "fb", "tw"]
                },
                {
                    "first": "Roger",
                    "last": "Craig",
                    "age": 68,
                    "nets": ["fb", "tw"]
                },
                {
                    "first": "Jane",
                    "last": "Murphy",
                    "age": 47,
                    "nets": ["ig", "tw"]
                }
            ]
        }"#;

    let p: Value = serde_json::from_str(data)
        .expect("Could not parse JSON");

    let get_item1 = p.rjson_get("phones.#").expect("Bruh");
    dbg!(get_item1);
    let get_item2 = p.rjson_get("phones.1").expect("Bruh");
    dbg!(get_item2);
    let get_item3 = p.rjson_get("friends.#.first").expect("Bruh");
    dbg!(get_item3);
    let get_item4 = p.rjson_get("friends.#.nets").expect("Bruh");
    dbg!(get_item4);
    let get_item5 = p.rjson_get("friends.#(age<50)#.first").expect("Bruh");
    dbg!(get_item5);
    let get_item6 = p.rjson_get(r#"friends.#(nets.#(=="ig"))#.first"#).expect("Bruh");
    dbg!(get_item6);
    let get_item7 = p.rjson_get(r#"friends.#(first%"D*").nets.2"#).expect("Bruh");
    dbg!(get_item7);
    let get_item8 = p.rjson_get(r#"friends.#(first!%"D*")#"#).expect("Bruh");
    dbg!(get_item8);
    let get_item9 = p.rjson_get(r#"friends.#(first%"*e")#"#).expect("Bruh");
    dbg!(get_item9);
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Key(String),
    Dot,
    Hashtag,
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanEqual,
    LessThan,
    LessThanEqual,
    Like,
    NotLike,
    String(String),
    Int(i64),
    Float(f64),
    QueryOnce(Vec<Token>),
    QueryAll(Vec<Token>),
    Null
}

#[derive(Debug)]
pub enum TokenError {
    UnexpectedEOF,
    UnexpectedCharacter(String),
    Msg(String)
}

trait Tokenize {
    type TokenIter;

    fn tokens(&mut self) -> Result<Self::TokenIter, TokenError>;

    fn tokenize_number(
        &mut self,
        first_char: char
    ) -> Result<Token, TokenError>;

    fn tokenize_query(&mut self) -> Result<Token, TokenError>;
    fn tokenize_key(&mut self, first_char: char) -> Result<Token, TokenError>;
    fn tokenize_string(&mut self) -> Result<Token, TokenError>;
}

trait Get: Sized {
    fn rjson_get(
        &self,
        expression: &str
    ) -> Result<Self, TokenError>;
}

impl Get for Value {
    fn rjson_get(
        &self,
        expression: &str
    ) -> Result<Self, TokenError>
    {
        let mut tokens = expression.chars().peekable().tokens()?.peekable();
        let mut curr_value: Value = self.to_owned();
        let mut in_array = false;

        while let Some(token) = tokens.next() {
            match token {
                Token::Dot => continue,
                Token::Key(key) => {
                    if in_array {
                        let mut temp_arr: Vec<&Value> = Vec::new();
                        match curr_value.as_array() {
                            Some(arr) => {
                                for item in arr {
                                    match item.as_object() {
                                        Some(object) => {
                                            let key_val = object.get(&key);
                                            match key_val {
                                                Some(key_value) =>
                                                temp_arr.push(key_value),
                                                None => return Err(
                                                    TokenError::Msg(
                                                        format!(
                                                            "Could not get key\nKey: {:?}", key
                                                        )
                                                    )
                                                )
                                            }
                                        },
                                        None => return Err(
                                            TokenError::Msg(
                                                "Could not get Object."
                                                    .to_string()
                                            )
                                        )
                                    }
                                }

                                curr_value = json!(temp_arr);
                            },
                            None => return Err(
                                TokenError::Msg(
                                    "Could not get array."
                                        .to_string()
                                )
                            )
                        }
                    } else {
                        curr_value = curr_value.get(&key)
                            .expect(format!("Could not index item: {}", key)
                            .as_str())
                            .to_owned();
                    }
                },
                Token::Hashtag => {
                    let arr = curr_value.as_array();
                    match arr {
                        Some(vec) => {
                            if tokens.peek().is_none() {
                                return Ok(json!(vec.len()))
                            }

                            in_array = true;
                            curr_value = Value::Array(vec.to_owned());
                        },
                        None => return Err(
                            TokenError::Msg(
                                "Could not get Array".to_string()
                            )
                        )
                    }
                },
                Token::Int(integer) => {
                    curr_value = curr_value.get(integer as usize)
                        .expect(
                        format!("Could not index array: {}", integer)
                        .as_str())
                        .to_owned();
                },
                Token::QueryOnce(query) => {
                    curr_value = parse_query(&query, curr_value.clone(), false, None)?;
                },
                Token::QueryAll(query) => {
                    in_array = true;
                    curr_value = parse_query(&query, curr_value.clone(), true, None)?;
                },
                _ => return Err(TokenError::Msg(token.to_string()))
            }
        }

        Ok(curr_value)
    }
}

fn parse_query(
    query: &Vec<Token>,
    curr_value: Value,
    hashtag: bool,
    key_value_hashmap: Option<HashMap<String, Value>>
) -> Result<Value, TokenError>
{
    #[allow(unused_assignments)]
    let mut query_arr : Vec<Value> = Vec::new();
    let mut return_raw_arr: Vec<Value> = Vec::new();
    let mut in_arr_val = false;
    let mut value_key_hash: HashMap<String, Value> = if key_value_hashmap.is_none() {
        HashMap::new()
    } else {
        in_arr_val = true;
        key_value_hashmap.unwrap()
    };

    match curr_value.clone().as_array() {
        Some(arr) => {
            query_arr = arr.to_owned();
        },
        None => return Err(
            TokenError::Msg(
                "Need Array for Query Parsing!".to_string()
            )
        )
    };

    let mut query_iter = query.into_iter().peekable();

    while let Some(token) = query_iter.next() {
        let peek_value: &Token;
        let peek_option = query_iter.peek();
        match peek_option {
            Some(peek) => {
                peek_value = peek;
            },
            None => peek_value = &Token::Null
        };

        match token {
            Token::Dot | Token::Hashtag => continue,
            Token::QueryOnce(query) => {
                assert!(query.len() == 2, "Expecting more in Query.");
                return_raw_arr = parse_query(
                    query,
                    Value::Array(return_raw_arr.clone()),
                    true,
                    Some(value_key_hash.clone()))
                    .expect("Can't parse inside Query.")
                    .as_array()
                    .unwrap()
                    .to_owned();
            },
            Token::Key(key) => {
                for query_value in &query_arr {
                    // Try to get the map
                    let hopefully_obj = query_value.as_object();
                    if hopefully_obj.is_none() {
                        return Err(TokenError::Msg(
                            "Could not get object".to_string()
                        ))
                    }

                    // Try to get a certain key in the map
                    let obj = hopefully_obj.unwrap();
                    let value = obj.get(key);
                    if value.is_none() {
                        return Err(TokenError::Msg(format!(
                            "Could not get Key: `{}`", key
                        )))
                    }

                    value_key_hash.insert(query_value.to_string(), value.unwrap().to_owned());
                }
            },
            Token::Equal | Token::NotEqual => {
                for (obj_value, obj_key_value) in &value_key_hash {
                    match peek_value {
                        Token::String(string) => {
                            if *token == Token::Equal {
                                if in_arr_val {
                                    for item in obj_key_value.as_array().unwrap() {
                                        if item.eq(string) {
                                            return_raw_arr.push(
                                                serde_json::from_str(
                                                    obj_value.as_str()
                                                ).expect("Could not parse JSON")
                                            );
                                        }
                                    }
                                } else {
                                    if obj_key_value.eq(string) {
                                        return_raw_arr.push(
                                            serde_json::from_str(
                                                obj_value.as_str()
                                            ).expect("Could not parse JSON")
                                        );
                                    }
                                }
                            } else {
                                if in_arr_val {
                                    for item in obj_key_value.as_array().unwrap() {
                                        if item.ne(string) {
                                            return_raw_arr.push(
                                                serde_json::from_str(
                                                    obj_value.as_str()
                                                ).expect("Could not parse JSON")
                                            );
                                        }
                                    }
                                } else {
                                    if obj_key_value.ne(string) {
                                        return_raw_arr.push(
                                            serde_json::from_str(
                                                obj_value.as_str()
                                            ).expect("Could not parse JSON")
                                        );
                                    }
                                }
                            }
                        },
                        Token::Float(float) => {
                            if *token == Token::Equal {
                                if in_arr_val {
                                    for item in obj_key_value.as_array().unwrap() {
                                        if item.eq(float) {
                                            return_raw_arr.push(
                                                serde_json::from_str(
                                                    obj_value.as_str()
                                                ).expect("Could not parse JSON")
                                            );
                                        }
                                    }
                                } else {
                                    if obj_key_value.eq(float) {
                                        return_raw_arr.push(
                                            serde_json::from_str(
                                                obj_value.as_str()
                                            ).expect("Could not parse JSON")
                                        );
                                    }
                                }
                            } else {
                                if in_arr_val {
                                    for item in obj_key_value.as_array().unwrap() {
                                        if item.ne(float) {
                                            return_raw_arr.push(
                                                serde_json::from_str(
                                                    obj_value.as_str()
                                                ).expect("Could not parse JSON")
                                            );
                                        }
                                    }
                                } else {
                                    if obj_key_value.ne(float) {
                                        return_raw_arr.push(
                                            serde_json::from_str(
                                                obj_value.as_str()
                                            ).expect("Could not parse JSON")
                                        );
                                    }
                                }
                            }
                        },
                        Token::Int(integer) => {
                            if *token == Token::Equal {
                                if in_arr_val {
                                    for item in obj_key_value.as_array().unwrap() {
                                        if item.eq(integer) {
                                            return_raw_arr.push(
                                                serde_json::from_str(
                                                    obj_value.as_str()
                                                ).expect("Could not parse JSON")
                                            );
                                        }
                                    }
                                } else {
                                    if obj_key_value.eq(integer) {
                                        return_raw_arr.push(
                                            serde_json::from_str(
                                                obj_value.as_str()
                                            ).expect("Could not parse JSON")
                                        );
                                    }
                                }
                            } else {
                                if in_arr_val {
                                    for item in obj_key_value.as_array().unwrap() {
                                        if item.eq(integer) {
                                            return_raw_arr.push(
                                                serde_json::from_str(
                                                    obj_value.as_str()
                                                ).expect("Could not parse JSON")
                                            );
                                        }
                                    }
                                } else {
                                    if obj_key_value.ne(integer) {
                                        return_raw_arr.push(
                                            serde_json::from_str(
                                                obj_value.as_str()
                                            ).expect("Could not parse JSON")
                                        );
                                    }
                                }
                            }
                        }
                        _ => return Err(
                            TokenError::Msg(
                                "Expected Integer, Float or String"
                                .to_string()
                            )
                        )
                    };
                }

                query_iter.next();
            },
            Token::Like | Token::NotLike => {
                if peek_value.is_num() {
                    return Err(TokenError::Msg("Expected String.".to_string()))
                }

                let string = {
                    match peek_value.as_string() {
                        Some(token_string) => token_string,
                        None => return Err(TokenError::Msg("Could not get string".to_string()))
                    }
                };

                #[allow(unused_assignments)]
                let mut pattern = String::new();
                let chars = string.chars();
                match chars.clone().nth(0) {
                    Some(first_char) => {
                        match first_char {
                            '*' => {
                                pattern = chars
                                    .skip_while(|x| *x == '*')
                                    .take_while(|x| *x != '*')
                                    .collect();
                                for (obj_value, obj_key_value) in &value_key_hash {
                                    if in_arr_val {
                                        for item in obj_key_value.as_array().unwrap() {
                                            match item.as_str() {
                                                Some(string) => {
                                                    if *token == Token::Like {
                                                        if string.ends_with(pattern.as_str()) {
                                                            return_raw_arr.push(
                                                                serde_json::from_str(
                                                                    obj_value.as_str()
                                                                ).expect("Could not parse JSON")
                                                            );
                                                        }
                                                    } else {
                                                        if !string.ends_with(pattern.as_str()) {
                                                            return_raw_arr.push(
                                                                serde_json::from_str(
                                                                    obj_value.as_str()
                                                                ).expect("Could not parse JSON")
                                                            );
                                                        }
                                                    }
                                                },
                                                None => return Err(TokenError::Msg(
                                                    "Expected String".to_string()
                                                ))
                                            }
                                        }
                                    } else {
                                        match obj_key_value.as_str() {
                                            Some(string) => {
                                                if *token == Token::Like {
                                                    if string.ends_with(pattern.as_str()) {
                                                        return_raw_arr.push(
                                                            serde_json::from_str(
                                                                obj_value.as_str()
                                                            ).expect("Could not parse JSON")
                                                        );
                                                    }
                                                } else {
                                                    if !string.ends_with(pattern.as_str()) {
                                                        return_raw_arr.push(
                                                            serde_json::from_str(
                                                                obj_value.as_str()
                                                            ).expect("Could not parse JSON")
                                                        );
                                                    }
                                                }
                                            },
                                            None => return Err(TokenError::Msg(
                                                "Expected String".to_string()
                                            ))
                                        }
                                    }
                                }
                            },
                            _ => {
                                pattern = chars.take_while(|x| *x != '*' ).collect();
                                for (obj_value, obj_key_value) in &value_key_hash {
                                    if in_arr_val {
                                        for item in obj_key_value.as_array().unwrap() {
                                            match item.as_str() {
                                                Some(string) => {
                                                    if *token == Token::Like {
                                                        if string.starts_with(pattern.as_str()) {
                                                            return_raw_arr.push(
                                                                serde_json::from_str(
                                                                    obj_value.as_str()
                                                                ).expect("Could not parse JSON")
                                                            );
                                                        }
                                                    } else {
                                                        if !string.starts_with(pattern.as_str()) {
                                                            return_raw_arr.push(
                                                                serde_json::from_str(
                                                                    obj_value.as_str()
                                                                ).expect("Could not parse JSON")
                                                            );
                                                        }
                                                    }
                                                },
                                                None => return Err(TokenError::Msg(
                                                    "Expected String".to_string()
                                                ))
                                            }
                                        }
                                    } else {
                                        match obj_key_value.as_str() {
                                            Some(string) => {
                                                if *token == Token::Like {
                                                    if string.starts_with(pattern.as_str()) {
                                                        return_raw_arr.push(
                                                            serde_json::from_str(
                                                                obj_value.as_str()
                                                            ).expect("Could not parse JSON")
                                                        );
                                                    }
                                                } else {
                                                    if !string.starts_with(pattern.as_str()) {
                                                        return_raw_arr.push(
                                                            serde_json::from_str(
                                                                obj_value.as_str()
                                                            ).expect("Could not parse JSON")
                                                        );
                                                    }
                                                }
                                            },
                                            None => return Err(TokenError::Msg(
                                                "Expected String".to_string()
                                            ))
                                        }
                                    }
                                }
                            }
                        }
                    },
                    None => return Err(
                        TokenError::Msg(
                            "Empty String.".to_string()
                        )
                    )
                }

                query_iter.next();
            },
            Token::GreaterThan | Token::GreaterThanEqual | Token::LessThan |
            Token::LessThanEqual => {
                if peek_value.is_string() {
                    return Err(
                        TokenError::Msg("Expected Float/Integer".to_string())
                    )
                }

                for (obj_value, obj_key_value) in &value_key_hash {
                    match peek_value {
                        Token::Float(float) => {
                            match obj_key_value.as_f64() {
                                Some(val_float) => {
                                    match token {
                                        Token::GreaterThan => {
                                            if in_arr_val {
                                                for item in obj_key_value.as_array().unwrap() {
                                                    match item.as_f64() {
                                                        Some(item_float) => {
                                                            if item_float > *float {
                                                                return_raw_arr.push(
                                                                    serde_json::from_str(
                                                                        obj_value.as_str()
                                                                    ).expect("Could not parse JSON")
                                                                );
                                                            }
                                                        },
                                                        None => return Err(TokenError::Msg(
                                                            "Expected Float".to_string()
                                                        ))
                                                    }
                                                }
                                            } else {
                                                if val_float > *float {
                                                    return_raw_arr.push(
                                                        serde_json::from_str(
                                                            obj_value.as_str()
                                                        ).expect("Could not parse JSON")
                                                    );
                                                }
                                            }
                                        },
                                        Token::GreaterThanEqual => {
                                            if in_arr_val {
                                                for item in obj_key_value.as_array().unwrap() {
                                                    match item.as_f64() {
                                                        Some(item_float) => {
                                                            if item_float >= *float {
                                                                return_raw_arr.push(
                                                                    serde_json::from_str(
                                                                        obj_value.as_str()
                                                                    ).expect("Could not parse JSON")
                                                                );
                                                            }
                                                        },
                                                        None => return Err(TokenError::Msg(
                                                            "Expected Float".to_string()
                                                        ))
                                                    }
                                                }
                                            } else {
                                                if val_float >= *float {
                                                    return_raw_arr.push(
                                                        serde_json::from_str(
                                                            obj_value.as_str()
                                                        ).expect("Could not parse JSON")
                                                    );
                                                }
                                            }
                                        },
                                        Token::LessThan => {
                                            if in_arr_val {
                                                for item in obj_key_value.as_array().unwrap() {
                                                    match item.as_f64() {
                                                        Some(item_float) => {
                                                            if item_float < *float {
                                                                return_raw_arr.push(
                                                                    serde_json::from_str(
                                                                        obj_value.as_str()
                                                                    ).expect("Could not parse JSON")
                                                                );
                                                            }
                                                        },
                                                        None => return Err(TokenError::Msg(
                                                            "Expected Float".to_string()
                                                        ))
                                                    }
                                                }
                                            } else {
                                                if val_float < *float {
                                                    return_raw_arr.push(
                                                        serde_json::from_str(
                                                            obj_value.as_str()
                                                        ).expect("Could not parse JSON")
                                                    );
                                                }
                                            }
                                        },
                                        Token::LessThanEqual => {
                                            if in_arr_val {
                                                for item in obj_key_value.as_array().unwrap() {
                                                    match item.as_f64() {
                                                        Some(item_float) => {
                                                            if item_float < *float {
                                                                return_raw_arr.push(
                                                                    serde_json::from_str(
                                                                        obj_value.as_str()
                                                                    ).expect("Could not parse JSON")
                                                                );
                                                            }
                                                        },
                                                        None => return Err(TokenError::Msg(
                                                            "Expected Float".to_string()
                                                        ))
                                                    }
                                                }
                                            } else {
                                                if val_float < *float {
                                                    return_raw_arr.push(
                                                        serde_json::from_str(
                                                            obj_value.as_str()
                                                        ).expect("Could not parse JSON")
                                                    );
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                },
                                None => return Err(
                                    TokenError::Msg(
                                        "Value is not Float."
                                        .to_string()
                                    )
                                )
                            }
                        },
                        Token::Int(integer) => {
                            match obj_key_value.as_i64() {
                                Some(val_int) => {
                                    match token {
                                        Token::GreaterThan => {
                                            if in_arr_val {
                                                for item in obj_key_value.as_array().unwrap() {
                                                    match item.as_i64() {
                                                        Some(item_int) => {
                                                            if item_int > *integer {
                                                                return_raw_arr.push(
                                                                    serde_json::from_str(
                                                                        obj_value.as_str()
                                                                    ).expect("Could not parse JSON")
                                                                );
                                                            }
                                                        },
                                                        None => return Err(TokenError::Msg(
                                                            "Expected Float".to_string()
                                                        ))
                                                    }
                                                }
                                            } else {
                                                if val_int > *integer {
                                                    return_raw_arr.push(
                                                        serde_json::from_str(
                                                            obj_value.as_str()
                                                        ).expect("Could not parse JSON")
                                                    );
                                                }
                                            }
                                        },
                                        Token::GreaterThanEqual => {
                                            if in_arr_val {
                                                for item in obj_key_value.as_array().unwrap() {
                                                    match item.as_i64() {
                                                        Some(item_int) => {
                                                            if item_int >= *integer {
                                                                return_raw_arr.push(
                                                                    serde_json::from_str(
                                                                        obj_value.as_str()
                                                                    ).expect("Could not parse JSON")
                                                                );
                                                            }
                                                        },
                                                        None => return Err(TokenError::Msg(
                                                            "Expected Float".to_string()
                                                        ))
                                                    }
                                                }
                                            } else {
                                                if val_int >= *integer {
                                                    return_raw_arr.push(
                                                        serde_json::from_str(
                                                            obj_value.as_str()
                                                        ).expect("Could not parse JSON")
                                                    );
                                                }
                                            }
                                        },
                                        Token::LessThan => {
                                            if in_arr_val {
                                                for item in obj_key_value.as_array().unwrap() {
                                                    match item.as_i64() {
                                                        Some(item_int) => {
                                                            if item_int < *integer {
                                                                return_raw_arr.push(
                                                                    serde_json::from_str(
                                                                        obj_value.as_str()
                                                                    ).expect("Could not parse JSON")
                                                                );
                                                            }
                                                        },
                                                        None => return Err(TokenError::Msg(
                                                            "Expected Float".to_string()
                                                        ))
                                                    }
                                                }
                                            } else {
                                                if val_int < *integer {
                                                    return_raw_arr.push(
                                                        serde_json::from_str(
                                                            obj_value.as_str()
                                                        ).expect("Could not parse JSON")
                                                    );
                                                }
                                            }
                                        },
                                        Token::LessThanEqual => {
                                            if in_arr_val {
                                                for item in obj_key_value.as_array().unwrap() {
                                                    match item.as_i64() {
                                                        Some(item_int) => {
                                                            if item_int <= *integer {
                                                                return_raw_arr.push(
                                                                    serde_json::from_str(
                                                                        obj_value.as_str()
                                                                    ).expect("Could not parse JSON")
                                                                );
                                                            }
                                                        },
                                                        None => return Err(TokenError::Msg(
                                                            "Expected Float".to_string()
                                                        ))
                                                    }
                                                }
                                            } else {
                                                if val_int <= *integer {
                                                    return_raw_arr.push(
                                                        serde_json::from_str(
                                                            obj_value.as_str()
                                                        ).expect("Could not parse JSON")
                                                    );
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                },
                                None => return Err(
                                    TokenError::Msg(
                                        "Value is not Integer."
                                        .to_string()
                                    )
                                )
                            }
                        },
                        _ => return Err(TokenError::UnexpectedCharacter(
                            token.to_string()
                        ))
                    }
                }

                query_iter.next();
            },
            _ => return Err(
                TokenError::UnexpectedCharacter(token.to_string())
            )
        };
    }

    if hashtag {
        return Ok(Value::Array(return_raw_arr))
    } else {
        match return_raw_arr.get(0) {
            Some(value) => {
                return Ok(value.to_owned())
            },
            None => return Err(TokenError::Msg(
                "Nothing to return.".to_string()
            ))
        }
    }
}

impl Token {
    fn is_string(&self) -> bool {
        match self {
            Token::String(_) => true,
            _ => false
        }
    }

    fn is_num(&self) -> bool {
        match self {
            Token::Float(_) | Token::Int(_) => true,
            _ => false
        }
    }

    fn as_string(&self) -> Option<String> {
        match self {
            Token::String(string) => Some(string.to_owned()),
            _ => None
        }
    }
}

impl<'a> Tokenize for Peekable<Chars<'a>> {
    type TokenIter = Box<dyn Iterator<Item = Token> + 'a>;

    fn tokens(&mut self) -> Result<Self::TokenIter, TokenError> {
        let mut tokens: Vec<Token> = Vec::new();

        while let Some(character) = self.next() {
            match character {
                '.' => tokens.push(Token::Dot),
                '#' => {
                    match self.peek() {
                        Some(character) => {
                            match character {
                                '(' => continue,
                                '.' => tokens.push(Token::Hashtag),
                                _ => return Err(
                                    TokenError::UnexpectedCharacter(character.to_string())
                                )
                            }
                        },
                        None => tokens.push(Token::Hashtag)
                    }
                },
                '(' => {
                    tokens.push(self.tokenize_query()?);
                },
                'a'..='z' | 'A'..='Z' => {
                    tokens.push(self.tokenize_key(character)?);
                },
                '0'..='9' => {
                    tokens.push(self.tokenize_number(character)?)
                },
                _ => return Err(
                    TokenError::UnexpectedCharacter(character.to_string())
                )
            };
        }

        Ok(Box::new(tokens.into_iter()))
    }

    fn tokenize_key(&mut self, first_char: char) -> Result<Token, TokenError> {
        let mut key = first_char.to_string();
        while let Some(&key_char) = self.peek() {
            match key_char {
                'a'..='z' | 'A'..='Z' | '*' => key.push(key_char),
                _ => break
            }
            self.next();
        }

        Ok(Token::Key(key))
    }

    fn tokenize_number(
        &mut self,
        first_char: char
    ) -> Result<Token, TokenError>
    {
        let mut number = first_char.to_string();
        let mut float = false;
        while let Some(&num_char) = self.peek() {
            match num_char {
                '0'..='9' => {
                    number.push(num_char);
                    self.next();
                },
                '.' => {
                    number.push(num_char);
                    float = true;
                    self.next();
                },
                _ => break
            }
        }

        if float {
            let float_num_result = number.parse::<f64>();
            match float_num_result {
                Ok(float) => return Ok(Token::Float(float)),
                Err(msg) => return Err(TokenError::Msg(
                    format!(
                        "Could not parse float.\nReason: {:?}", msg
                    )
                ))
            };
        } else {
            let int_num_result = number.parse::<i64>();
            match int_num_result {
                Ok(integer) => return Ok(Token::Int(integer)),
                Err(msg) => return Err(TokenError::Msg(
                    format!(
                        "Could not parse int.\nReason: {:?}", msg
                    )
                ))
            };
        }
    }

    fn tokenize_string(&mut self) -> Result<Token, TokenError> {
        let mut string = String::new();
        let mut slash = false;

        #[allow(irrefutable_let_patterns)]
        while let option_string_char = self.next() {
            match option_string_char {
                Some(string_char) => {
                    match string_char {
                        '\\' => {
                            string.push(string_char);
                            slash = true;
                        },
                        '"' => {
                            if slash {
                                slash = false;
                                string.push('"');
                            } else {
                                break
                            }
                        },
                        _ => string.push(string_char)
                    };
                },
                None => return Err(
                    TokenError::UnexpectedEOF
                )
            };
        }

        Ok(Token::String(string))
    }

    fn tokenize_query(&mut self) -> Result<Token, TokenError> {
        let mut all_match = false;
        let mut query: Vec<Token> = Vec::new();

        while let Some(character) = self.next() {
            match character {
                '.' | '(' => continue,
                '#' => query.push(self.tokenize_query()?),
                ')' => {
                    match self.peek() {
                        Some(character) => {
                            match character {
                                '#' => {
                                    all_match = true;
                                    self.next();
                                    break;
                                },
                                _ => break
                            }
                        },
                        None => break
                    };
                },
                'a'..='z' | 'A'..='Z' => {
                    query.push(self.tokenize_key(character)?);
                },
                '0'..='9' => {
                    query.push(self.tokenize_number(character)?)
                },
                '"' => {
                    query.push(self.tokenize_string()?);
                },
                '%' => query.push(Token::Like),
                '=' => {
                    match self.peek() {
                        Some(peek_char) => {
                            match peek_char {
                                '=' => { query.push(Token::Equal); self.next(); },
                                _ => return Err(TokenError::UnexpectedCharacter
                                    (format!("={}", peek_char)))
                            };
                        },
                        None => return Err(TokenError::UnexpectedEOF)
                    };
                },
                '!' => {
                    match self.peek() {
                        Some(peek_char) => {
                            match peek_char {
                                '=' => query.push(Token::NotEqual),
                                '%' => query.push(Token::NotLike),
                                _ => return Err(TokenError::UnexpectedCharacter(format!("!{}", peek_char)))
                            };
                        },
                        None => return Err(TokenError::UnexpectedEOF)
                    };
                    self.next();
                },
                '<' => {
                    match self.peek() {
                        Some(peek_char) => {
                            match peek_char {
                                '=' => {
                                    query.push(Token::LessThanEqual);
                                    self.next();
                                },
                                _ => {
                                    query.push(Token::LessThan);
                                }
                            };
                        },
                        None => return Err(TokenError::UnexpectedEOF)
                    };
                },
                '>' => {
                    match self.peek() {
                        Some(peek_char) => {
                            match peek_char {
                                '=' => {
                                    query.push(Token::GreaterThanEqual);
                                    self.next();
                                },
                                _ => {
                                    query.push(Token::GreaterThan);
                                }
                            };
                        },
                        None => return Err(TokenError::UnexpectedEOF)
                    };
                },
                _ => return Err(
                    TokenError::UnexpectedCharacter(
                        character.to_string()
                    )
                )
            }
        }

        if all_match {
            Ok(Token::QueryAll(query))
        } else {
            Ok(Token::QueryOnce(query))
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Key(key) => write!(f, "Key: {}", key),
            Token::Dot => write!(f, "."),
            Token::Hashtag => write!(f, "#"),
            Token::Equal => write!(f, "=="),
            Token::NotEqual => write!(f, "!="),
            Token::GreaterThan => write!(f, ">"),
            Token::GreaterThanEqual => write!(f, ">="),
            Token::LessThan => write!(f, "<"),
            Token::LessThanEqual => write!(f, "<="),
            Token::Like => write!(f, "%"),
            Token::NotLike => write!(f, "!%"),
            Token::String(string) => write!(f, "String: {}", string),
            Token::Int(int) => write!(f, "Integer: {}", int),
            Token::Float(float) => write!(f, "Float: {}", float),
            Token::QueryAll(query) | Token::QueryOnce(query) => write!(f, "Query: {}",
                query.iter().map(|token| token.to_string()).collect::<String>()
            ),
            Token::Null => write!(f, "NULL")
        }
    }
}

impl fmt::Display for TokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenError::UnexpectedEOF => write!(f, "Unexpected End of File!"),
            TokenError::UnexpectedCharacter(unexpected) => write!(
                f, "Unexpected Character! ~{}~", unexpected
            ),
            TokenError::Msg(msg) => write!(
                f, "Error Message: ~{}~", msg
            )
        }
    }
}
