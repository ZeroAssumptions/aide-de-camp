# Upgrading to v0.3.0

## Breaking Changes

### 1. Bincode → JSON Serialization

Job payloads are now serialized as JSON instead of bincode for better debuggability and stability.

#### Impact

- **Existing SQLite databases** with bincode-serialized payloads are **incompatible** with v0.3.0
- Job payloads stored in the `adc_queue` and `adc_dead_queue` tables cannot be deserialized

#### Migration Options

##### Option 1: Fresh Start (Recommended for Development)

Delete your SQLite database file and restart with an empty queue:

```bash
rm your-database.db
```

##### Option 2: Drain Queue Before Upgrading (Production)

1. Stop scheduling new jobs
2. Wait for all existing jobs to complete or fail
3. Verify queues are empty:
   ```sql
   SELECT COUNT(*) FROM adc_queue;
   SELECT COUNT(*) FROM adc_dead_queue;
   ```
4. Upgrade to v0.3.0
5. Resume normal operations

##### Option 3: Manual Data Migration

If you have critical queued jobs that must be preserved, you'll need to:

1. Export job data using v0.2.0
2. Upgrade to v0.3.0
3. Re-queue jobs with the new serialization format

### 2. Code Changes Required

Update your payload types to use serde traits instead of bincode:

```rust
// Before (v0.2.0)
use aide_de_camp::prelude::{Encode, Decode};

#[derive(Encode, Decode)]
struct MyPayload {
    field: String,
}

// After (v0.3.0)
use aide_de_camp::prelude::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct MyPayload {
    field: String,
}
```

### 3. Development Environment Changes

The project now uses **devenv** instead of flake.nix for a simpler development experience:

```bash
# Install devenv if needed
nix profile install nixpkgs#devenv

# Enter development environment
devenv shell

# Or use direnv
direnv allow
```

### 4. Dependency Updates

- **sqlx**: Updated from 0.6.2 to 0.8.x
  - Offline mode workflow has changed in sqlx 0.8
  - Run `DATABASE_URL=sqlite:your.db cargo sqlx prepare` to regenerate offline data
- All other dependencies updated to latest compatible versions

## Benefits of Upgrading

### Why JSON?

- **Human-readable** payloads make debugging much easier
- **Stable API** - no more release candidate dependencies (bincode was 2.0.0-rc.1)
- **Standard ecosystem** - serde is the de facto serialization library in Rust
- **Better error messages** when deserialization fails

### Performance Impact

JSON serialization is slightly larger and slower than bincode, but for a job queue use case:
- The difference is negligible (microseconds per job)
- The debugging benefits far outweigh the minimal performance cost
- Storage increase is typically <20% for most payload types

## Post-Upgrade Checklist

- [ ] Update all `#[derive(Encode, Decode)]` to `#[derive(Serialize, Deserialize)]`
- [ ] Update imports from `bincode::{Encode, Decode}` to `serde::{Serialize, Deserialize}`
- [ ] Handle existing database data (drain, delete, or migrate)
- [ ] Run tests to ensure payloads serialize/deserialize correctly
- [ ] If using sqlx macros: regenerate offline data with `cargo sqlx prepare`

## Need Help?

If you encounter issues during migration:

1. Check that all payload types implement `Serialize + Deserialize`
2. Verify your database is either empty or drained before upgrading
3. Ensure `serde` and `serde_json` dependencies are present
4. Open an issue at https://github.com/zero-assumptions/aide-de-camp/issues
