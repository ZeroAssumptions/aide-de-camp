<h1 align="center">Aide-De-Camp</h1>
<div align="center">
 <strong>
   💂 Aide-De-Camp
 </strong>
 
</div>

<br />


<div align="center">
  <h4>
    <a href="#Install">
      Install
    </a>
    <span> | </span>
    <a href="#Usage">
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

![crates.io](https://img.shields.io/crates/v/aide-de-camp.svg)
![pr-build](https://github.com/ZeroAssumptions/aide-de-camp/actions/workflows/pr-build.yaml/badge.svg)
[![CLA assistant](https://cla-assistant.io/readme/badge/ZeroAssumptions/aide-de-camp)](https://cla-assistant.io/ZeroAssumptions/aide-de-camp)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
 
<br />

Aide-De-Camp is a backend agnostic delayed job queue. Very similar to [`girl_friday`](https://github.com/mperham/girl_friday) from the Ruby world.

Notable features:

- **Asynchronous**. Built from the ground-up using async/await for maximum concurrency.
- **Backend agnostic**[^1]. This crate won't force you to run any additional software you don't want to.
- **Flexible scheduling**. Schedule your job to run right now, in time relative to now or at a specific future time.
- **Binary Payloads**. Job payloads serialized with bincode for maximum speed and space efficiency.
- **Concurrency limits.** Specify desired concurrency for each runner.
- **Job Router**. Run as many job types as you want and limit which runners can process which jobs.
- **Cross Platform**. Runs anywhere rust has standard lib.
- **Traced**. All important functions are [instrumented](https://github.com/tokio-rs/tracing).

## Getting started

### Install

Pick a backend[^1] that suits you and add following to your `Cargo.toml`:

```toml
# Cargo.toml

[dependencies]
aide-de-camp = "0.2.0"
aide-de-camp-sqlite = "0.2.0"  # Or any other available backend
```

### Usage

`aide-de-camp-sqlite` has a pretty detailed example of how to use this crate.

## Components

- **core**. Traits and error types meant to be used by aide-de-camp ecosystem.
- **runner**. Optional module (`runner` feature enabled by default) that contains default implementations of runner, job router and types required for their work.

[^1]: Currently, only SQLite supported (`aide-de-camp-sqlite`), but PostgreSQL backend is coming soon!

## License

Choose licensing under either of the following for your use case:

-   Apache License, Version 2.0
    ([LICENSE-APACHE](https://github.com/ZeroAssumptions/aide-de-camp/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
-   MIT license
    ([LICENSE-MIT](https://github.com/ZeroAssumptions/aide-de-camp/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
    
## Blog Posts

- [Build a Job Queue with Rust Using Aide-De-Camp (Part 1)](https://dev.to/zeroassumptions/build-a-job-queue-with-rust-using-aide-de-camp-part-1-4g5m)
- [Build a Job Queue with Rust Using Aide-De-Camp (Part 2)](https://dev.to/zeroassumptions/build-a-job-queue-with-rust-using-aide-de-camp-part-2-993)

## Contribution

Contributions are subject to CLA (Contributor License Agreement). Any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
