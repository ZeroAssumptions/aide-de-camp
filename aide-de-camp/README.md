<h1 align="center">Aide-De-Camp</h1>
<div align="center">
 <strong>
   💂 Aide-De-Camp
 </strong>
</div>

<br />


<div align="center">
  <h4>
    <a href="#install">
      Install
    </a>
    <span> | </span>
    <a href="#usage">
      Usage
    </a>
    <span> | </span>
    <a href="https://docs.rs/aide-de-camp">
      Docs
    </a>
  </h4>
</div>

<br />

<div align="center">
  <small>Built with ❤️ by <a href="https://zeroassumptions.dev">Zero Assumptions</a></small>
</div>

<br />

Aide-De-Camp is a backend agnostic delayed job queue. Very similar to [`girl_friday`](https://github.com/mperham/girl_friday) from Ruby world.

Notable features:

- **Asynchronous**. Built from the ground-up using async/await for maximum concurrency.
- **Backend agnostic**[^1]. Not forcing you to run any additional software you don't want to.
- **Flexible scheduling**. Schedule your job to run "now", in time relative to now or at specific future time.
- **Binary Payloads**. Job payloads serialized with bincode for maximum speed and space efficiency.
- **Concurrency limits.** Specify desired concurrency for each runner.
- **Job Router**. Run as many job types as you want and limit which runners can process which jobs.
- **Cross Platform**. Runs anywhere rust has standard lib.
- **Traced**. All important functions are [instrumented](https://github.com/tokio-rs/tracing).

## Getting started

### Installing

Pick a backend[^1] that suits you and add following to

```toml
# Cargo.toml

[dependencies]
aide-de-camp = "0.1.0"
aide-de-camp-sqlite = "0.1.0"  # Or any other avaiable backend
```



### Usage

`aide-de-camp-sqlite` has pretty detailed example of how to use this crate.

## Components

-**core**. Traits and error types meant to be used by aide-de-camp eco-system.
-**runner**. Optional module (`runner` feature enabled by default) that contains default implementations of runner, job router and types required for their work.

[^1]: Currently only SQLite supported (`aide-de-camp-sqlite`), but PostgreSQL coming soon!

## License

Licensed under either of

-   Apache License, Version 2.0
    ([LICENSE-APACHE](https://github.com/ZeroAssumptions/aide-de-camp/blob/main/LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
-   MIT license
    ([LICENSE-MIT](https://github.com/ZeroAssumptions/aide-de-camp/blob/main/LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
