use leptos::prelude::*;

use web::SimpleCounter;

pub fn main() {
    mount_to_body(|| {
        view! { <SimpleCounter initial_value=0 step=1/> }
    })
}