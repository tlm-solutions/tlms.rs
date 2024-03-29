# TLMS Rust Crate

[![built with nix](https://builtwithnix.org/badge.svg)](https://builtwithnix.org)

This crate contains all the reusable code for our vehicle tracking efforts. To
use, just drop it into your `Cargo.toml`.

## Building and Hacking

### With Nix (aka easy way)

This flake provides a devshell, which exposes all the dependencies
automatically. Just run `nix develop` anywhere in the repo.

### Without Nix

Just install the dependencies, build is done by cargo.

List of dependencies:
```
grpc
protobuf
websocketpp
pkg-config
postgresql_14
openssl
diesel-cli
```

## Documentation

Run `cargo doc --all-features --open` in a nix devshell, hosted version coming
soon-ish ;).

## Features 

List of rust features this crate exposes: `schema`, `management`, `locations`,
`telegrams`, `measurements`, `receivers`, `trekkie`, `gps`

## Entity Relationship diagram

```mermaid
erDiagram
	gps_points {
		BIGSERIAL id PK
		UUID trekkie_run FK "trekkie_runs(id)"
		TIMESTAMP timestamp
		DOUBLE lat
		DOUBLE lon
		DOUBLE elevation         "optional"
		DOUBLE accuracy          "optional"
		DOUBLE vertical_accuracy "optional"
		DOUBLE bearing           "optional"
		DOUBLE speed             "optional"
	}

	r09_telegrams {
		BIGSERIAL id PK
		TIMESTAMP time
		UUID station FK "stations(id)"
		BIGINT r09_type
		INT delay              "optional"
		INT reporting_point
		INT junction
		SMALLINT direction
		SMALLINT request_status
		SMALLINT priority           "optional"
		SMALLINT direction_request  "optional"
		INT line               "optional"
		INT run_number         "optional"
		INT destination_number "optional"
		INT train_length       "optional"
		INT vehicle_number     "optional"
		SMALLINT operator           "optional"
		BIGINT region FK "regions(id)"
	}

	r09_transmission_locations {
		BIGSERIAL id PK
		BIGINT region FK "regions(id)"
		INT reporting_point
		DOUBLE lat
		DOUBLE lon
        BOOLEAN ground_truth
	}

	r09_transmission_locations_raw {
		BIGSERIAL id PK
		BIGINT region FK "regions(id)"
		INT reporting_point
		DOUBLE lat
		DOUBLE lon
		UUID trekkie_run FK "trekkie_runs(id)"
		UUID run_owner FK "users(id)"
	}

	raw_telegrams {
		BIGSERIAL id PK
		TIMESTAMP time
		UUID station FK "stations(id)"
		BIGINT telegram_type
		BYTEA data
	}

	regions {
		BIGSERIAL id PK
		TEXT name
		TEXT transport_company
		TEXT regional_company   "optional"
		BIGINT frequency          "optional"
		BIGINT r09_type           "optional"
		INT encoding           "optional"
		BOOLEAN deactivated
        FLOAT lat
        FLOAT lon
        FLOAT zoom
        FLOAT work_in_progress
	}

    region_statistics {
        BIGINT id PK "regions(id)"
	    TIMESTAMP last_updated
	    BIGINT total_telegrams
	    BIGINT month_telegrams
	    BIGINT week_telegrams
	    BIGINT day_telegrams
	    BIGINT total_gps
	    BIGINT month_gps
	    BIGINT week_gps
	    BIGINT day_gps
    }

	stations {
		UUID id PK
		VARCHAR(36) token                 "optional"
		TEXT name
		DOUBLE lat
		DOUBLE lon
		BIGSERIAL region FK "regions(id)"
		UUID owner FK "users(id)"
		BOOLEAN approved
		BOOLEAN deactivated
		BOOLEAN public
		INT radio                    "optional"
		INT architecture             "optional"
		INT device                   "optional"
		DOUBLE elevation              "optional"
		INT antenna                  "optional"
		TEXT telegram_decoder_version "optional"
		TEXT notes                    "optional"
        UUID organization FK "organizations(id)"
	}

    station_statistics {
        UUID id PK "stations(id)"
        TIMESTAMP last_updated
        BIGINT total_telegrams
        BIGINT month_telegrams
        BIGINT week_telegrams
        BIGINT day_telegrams
    }

	trekkie_runs {
		TIMESTAMP start_time
		TIMESTAMP end_time
		INT line
		INT run
		BIGSERIAL region FK "regions(id)"
		UUID owner FK "users(id)"
		BOOLEAN finished
		UUID id PK
        BOOLEAN correlated
	}

	users {
		UUID id PK
		TEXT name          "optional"
		TEXT email         "optional"
		VARCHAR(100) password
		INT email_setting "optional"
		BOOLEAN deactivated
        BOOLEAN admin
	}

    user_statistics {
        UUID id PK "users(id)"
        TIMESTAMP last_updated
        BIGINT total_gps
        BIGINT month_gps
        BIGINT week_gps
        BIGINT day_gps
    }

    organizations {
		UUID id PK
		TEXT name
        BOOLEAN public
        UUID owner FK "users(id)"
        BOOLEAN deactivated
	}

    org_users_relations {
        UUID id PK
        UUID organization FK "organizations(id)"
        UUID user_id FK "users(id)"
        INT role
    }


  r09_transmission_locations }|--|| regions : "has"
  region_statistics ||--o| regions : "statistics"
  r09_telegrams }|--|| regions : "received in"
  r09_transmission_locations_raw }|--|| regions : ""
  stations }|--|| regions : "contains"

  gps_points }|--|| trekkie_runs : "contains"
  trekkie_runs }|--|| users : "from"

  r09_transmission_locations_raw }|--|| users : ""
  trekkie_runs }|--|| regions : "in"
  r09_transmission_locations_raw }|--|| trekkie_runs : "contains"

  r09_telegrams }|--|| stations : "received"
  raw_telegrams }|--|| stations : "received"
  stations }|--|| organizations: "belongs"
  stations }|--|| users : "owns"
  organizations }|--|| users : "owns"
  org_users_relations }|--|| users : "has role"
  org_users_relations }|--|| organizations : "associated key"
  station_statistics ||--o| stations : "statistics"
  user_statistics ||--o| users : "statistics"

```
