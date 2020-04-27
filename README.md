# rust-statemap

rust-statemap is a rust library to assist in the creation of
[statemaps](https://github.com/joyent/statemap).

This is still a prototype / proof-of-concept.

Statemap terminology can be confusing and overloaded. 'Statemap' may refer to:
- An SVG file.
- A bunch of JSON data in a specific format.
- A program that turns JSON into an SVG.

At the lowest layer a statemap is just a JSON protocol. The statemap tool is one
such tool that may be used to render the protocol and chooses to do so in an SVG
format.

This project has two goals:
1) Define the protocol (e.g. the set of JSON structures) separately from the
  tool(s) that consume the protocol.
2) Provide a library for Rust programs that want to serialize/deserialize data
  into/from the protocol.

The tools that consume the statemap protocol (e.g. joyent/statemap) may use
the structures defined in this library in the future to ensure the protocol and
rendering tools do not get out of sync. Currently the structures are copied
nearly verbatim from the joyent/statemap source.

Alternatively, this library could be bundled with the joyent/statemap tool
instead. Either approach is fine as long as the abstractions aren't leaky.

## Example usage

This is a simple, minimal example.

```rust
use statemap::Statemap;

/* Give the statemap a name and (optionally) hostname. */
let mut statemap = Statemap::new("demo statemap", Some("localhost".to_owned()), None);

/* Set a state for the given entity at the current time. */
statemap.set_state("main", "working", None, Utc::now());

/* Optionally set a specific color for the given state. */
statemap.set_state_color("working", "red");

/*
 * Serialize the statemap. This data may then be sent through the statemap tool.
 */
for state in statemap {
	println!("{}", state);
}

```

## Notes

Not all of the statemap protocol is supported by this library. At this time only
states may be set. More advanced behavior like state tagging, events, and
descriptions are not yet implemented.

The API is under development. We had some discussion within Joyent about some
different ways we could make the API more easy to use:
- `state!` macro
  - For example, a user may do something like
    `state!("state_entity", "state_name")`
- `#[state]` attribute
  - For example, wrapping a function or module in a `#[state]` attribute would
    automatically set a state on entry.

We should also consider what changes should be made to the current API to make
it more idiomatic for Rust programs.

## Statemap protocol moans and niggles

There are a few things that would make the statemap protocol easier to
serialize:
- Make most metadata optional (e.g. remove the required header):
  - A beginning timestamp may be provided if the user so chooses.
  - Make state metadata (e.g. color) an optional top-level protocol member.
- Make title metadata (e.g. title, hostname) a top-level protocol member.
  - Title may be required, hostname remains optional, etc.
- Allow each state to specify its absolute time rather than time relative to the
  first state.
- Replace StatemapDatum.state numbers with string names.
- Allow arbitrary ordering of metadata (e.g. title or state color metadata may 
  be the first line, the last line, or anywhere else).
  - State timestamps must remain in ascending order for a given entity.
