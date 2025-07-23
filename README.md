# SNOW API
Unofficial Rust API Bindings for ServiceNow.

*Disclaimer: This crate is not affiliated with ServiceNow or any other related
company. Use at your own risk*

The `snow_api` crate provides easy support for interacting with common
APIs like the Table API in ServiceNow.

## Adding to Cargo.toml and Imports
Add the following to your Cargo.toml dependencies:
```toml
[dependencies]
snow_api = {version = "1.0"}
```

Then the crate features can be access by importing the crates prelude:
```rs
use snow_api::prelude::*;
```

## Example Usage
```rs
use snow_api::prelude::*;

#[glideable]
struct User {
    location: GlideReference,
    first_name: String,
    last_name: String,
}

fn main() {
    let mut user_gr: GlideRecord<User> = GlideRecord::new("sys_user").unwrap();
    user_gr.set_limit(1);
    let _ = user_gr.query();
    let mut sys_id = String::new();

    if let Some(mut user) = user_gr.next() {
        println!("{}", user.first_name);
        sys_id = user.sys_id.to_owned();
        user.first_name = String::from("just a test");
        user.update(&user_gr);
    }

    if let Some(user) = user_gr.get(&sys_id) {
        println!("{}", user.first_name);
    }
}
```

## Handling Credentials
The easiest way to handle api credentials is by simply adding the following
environment variables to your system:
```bash
export SNOW_API_USER="YOURUSERNAME"
export SNOW_API_PASSWD="YOURPASSWORD"
export SNOW_API_INSTANCE="#########.service-now.com"
```
The rest is automatically handled. If you require a more custom/dynamic form
of credential handling, a GlideRecord struct can be configured with the following
method:
```rs
let basic = CredentialType::Basic {
    user_name: String::from("YOURUSERNAME"), passwd: String::from("YOURPASSWORD")
};
//or
let token = CredentialType::Token(String::from("SOME TOKEN"))
let instance = String::from("#########.service-now.com");
let config = GlideRecordConfig::new_with_credentials(instance, basic);

let user_gr: GlideRecord<User> = GlideRecord::new_with_configuration("sys_user", config);
```

## The Glideable Trait and Attribute
The Glideable trait guarantees a few things:

- A struct that is Glideable should Serializable/Deserializable with serde.
- A struct that is Glideable will always have a `sys_id` field.
- A struct that is Glideable will always be able to be inserted/updated/deleted.

Any struct that can usually derive Serialized and Deserialized with serde should
be able to have the glideable attribute applied as follows:

```rs
#[glideable]
struct SomeStruct1 {
    //sys_id field will be added if not included.
    some_field: String,
}

#[glideable]
struct SomeStruct2 {
    sys_id: String,
    some_field: String,
}
```

## The GlideRecord Struct

A glide record in rust works a bit different from ServiceNow's glide record
simply because rust requires a bit more of a type guarantee and we need to be
able to handle table inheritance as easily as possible. The following is how
one initializes a GlideRecord:
```rs
use snow_api::prelude::*;

#[glideable]
struct User {
    first_name: String,
    last_name: String,
}
fn main() {
    //You must declare the Structure being used, i.e. GlideRecord<User>
    //The struct must be have the Glideable trait
    let user_gr: GlideRecord<User> = GlideRecord::new("sys_user").unwrap();
}
```

### Handling Updates and Inserts

Updating a record is a bit different. We first pull the record and modify it which will feel
similar to normal glide record usage. However, when we call a method like `update`, we
instead pass a reference of the original GlideRecord struct to the update method:
```rs
let user_gr: GlideRecord<User> = GlideRecord::new("sys_user").unwrap();
if let Some(user) = user_gr.get("some_sys_id") {
    //modify the user
    user.update(&user_gr);
}
```

This may feel a bit odd, but unlike in servicenow, a gliderecord in this case
is separate from the actual record structure.

### Working with GlideReferences
Your struct can include a `GlideReference` for a field like so:
```rs
#[glideable]
struct User {
    location: GlideReference,
}
```

A `GlideReference` struct has two convience functions:
- `as_glide_record()`: returns a GlideRecord with a query of `sys_id=<the records id>`
- `as_item()`: returns the actual record the glide record would return

These two functions make it fairly simple to dot walk through things. Just be aware
that both cause an API call to get the record when used.

### Current Support
- [x] **Namespace:** NOW   (Main APIs)
    - [x] Table API
        - [x] Retrieve records from a table  (GET)
        - [x] Create a record  (POST)
        - [x] Retrieve a record  (GET)
        - [x] Modify a record  (PUT)
        - [x] Delete a record  (DELETE)
        - [x] Update a record  (PATCH)
