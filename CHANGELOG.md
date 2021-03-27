# v0.3.2
___
### Fix
    - documentation updates

# v0.3.0
___
### Feature
> - added cargo features
> 
>   **rt_tokio** (*build with tokio async runtime and without sqlx db migration support*)
> 
>   **rt_tokio_migrate** (*build with tokio async runtime and sqlx db migration support*)

# v0.2.3
___
### Dependencies
    - added sqlx

### Fix
    - added start timeout

### Feature
    - added PgEmbed::create_database(name)

# v0.2.2
___

### Features
- added port setting to PgSettings

# v0.2.0
___

- switched from async-std to tokio
- switched from surf to reqwest

