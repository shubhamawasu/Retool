# Local Development Setup

## Environment configuration

This app defaults to **production behavior** (e.g. `Secure` session cookies)
unless explicitly told otherwise. To run locally over plain HTTP, you need a
`.env` file that opts into development mode.

### 1. Create your `.env` file

In the project root, create a file named `.env`:

```
APP_ENV=development
```

This file is gitignored and should never be committed — it only exists on
your local machine.

If it doesn't, add it before creating your `.env` file.

### 2. Run the app

```bash
cargo run
```

If http is not working, it means
`APP_ENV` wasn't picked up — double check:
- `.env` exists in the project directory
- It contains exactly `APP_ENV=development` (no quotes, no extra spaces)
- You don't have `APP_ENV` already set in your shell environment (a shell-level
  `export APP_ENV=production` will override `.env`, since env vars set before
  the process starts take precedence)

### Why production is the default

If `APP_ENV` is unset (e.g. forgotten in a deploy environment), the app
fails safe into production behavior — cookies get `Secure` set, rather than
silently running an insecure configuration. You have to explicitly opt into
the relaxed local-dev behavior via `.env`, not the other way around.

### Notes

- `.env` is for **local development only**. Production environments should
  set `APP_ENV=production` (or leave it unset) through your actual deployment
  mechanism (Docker env vars, systemd unit, hosting platform config, etc.) —
  never via a checked-in `.env` file.
- Valid values for `APP_ENV` are `production` and `development`. Any other
  value will cause the app to panic on startup with a clear error, rather
  than silently falling back to an unexpected mode.
