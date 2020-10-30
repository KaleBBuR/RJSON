# RJSON is a simple package that ties in with Serde JSON to get your JSON items easily!

You guys should totally check out [GJSON](https://github.com/tidwall/gjson) though! It's a very nice package for golang.

## Examples
``` rust
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
    /*
    Return: Integer: 2
     */

    let get_item2 = p.rjson_get("phones.1").expect("Bruh");
    dbg!(get_item2);
    /*
    Return: String: "+44 2345678"
     */

    let get_item3 = p.rjson_get("friends.#.first").expect("Bruh");
    dbg!(get_item3);
    /*
    Return: Array: [
        String: "Dale",
        String: "Roger",
        String: "Jane"
    ]
     */

    let get_item4 = p.rjson_get("friends.#.nets").expect("Bruh");
    dbg!(get_item4);
    /*
    Return: Array: [
        Array: [
            String: "ig",
            String: "fb",
            String: "tw",
        ],
        Array: [
            String: "fb",
            String: "tw",
        ],
        Array: [
            String: "ig",
            String: "tw",
        ]
    ]
     */

    let get_item5 = p.rjson_get("friends.#(age<50)#.first").expect("Bruh");
    dbg!(get_item5);
    /*
    Return: Array: [
        String: "Jane",
        String: "Dale"
    ]
     */

    let get_item6 = p.rjson_get(r#"friends.#(nets.#(=="ig"))#.first"#).expect("Bruh");
    dbg!(get_item6);
    /*
    Return: Array: [
        String: "Jane",
        String: "Dale"
    ]
     */

    let get_item7 = p.rjson_get(r#"friends.#(first%"D*").nets.2"#).expect("Bruh");
    dbg!(get_item7);
    /*
    Return: String: "tw"
     */

    let get_item8 = p.rjson_get(r#"friends.#(first!%"D*")#"#).expect("Bruh");
    dbg!(get_item8);
    /*
    Return: Array: [
        Object: {
            "age": Number: 47,
            "first": String: "Jane",
            "last": String: "Murphy",
            "nets": Array: [
                String: "ig",
                String: "tw"
            ]
        },
        Object: {
            "age": Number: 68,
            "first": String: "Roger",
            "last": String: "Craig",
            "nets": Array: [
                String: "fb",
                String: "tw"
            ]
        }
    ]
     */

    let get_item9 = p.rjson_get(r#"friends.#(first%"*e")#"#).expect("Bruh");
    dbg!(get_item9);

    /*
    Return: Array: [
        Object: {
            "age": Number: 44,
            "first": String: "Dale",
            "last": String: "Murphy",
            "nets": Array: [
                String:  "ig",
                String: "fb",
                String: "tw",
            ],
        },
        Object: {
            "age": Number: 47,
            "first": String: "Jane",
            "last": String: "Murphy",
            "nets": Array: [
                String: "ig",
                String: "tw",
            ],
        },
    ]
     */
}
```

# TODO
------
* Impliment Modifers
```
@reverse
@ugly
@pretty
@this
@valid
@flatten
@join
```
* Custom Modifiers
* Get Type
* Raw
* JSON Lines ```(..)```
* Check for esistence of a value
* Optimize the holy hell out of this crappy code