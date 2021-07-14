# v0.5.2
___
### Fix
- Password was created at wrong destination
- stopping db on drop fix

# v0.5.1
___
### Fix
- **PgEmbed**'s ***stop_db()*** did not execute on drop
- Multiple concurrent **PgEmbed** instances tried each to download the same resources when being setup

# v0.5.0
___
### Feature
> - Caching postgresql binaries
>    
>   Removed **executables_dir** attribute from **PgSettings**
> 
>   The downloaded postgresql binaries are now cached in the following directories:
>   
>   - On Linux:
>     
>     **$XDG_CACHE_HOME/pg-embed**
> 
>     or 
> 
>     **$HOME/.cache/pg-embed**
>   - On Windows: 
>     
>     **{FOLDERID_LocalAppData}/pg-embed**
>   - On MacOS:
> 
>     **$HOME/Library/Caches/pg-embed**
> 
>   Binaries download only happens if cached binaries are not found
> - Cleaner logging
>   
>   Logging is now done with the **log** crate. 
>   
>   In order to produce log output a logger implementation compatible with the facade has to be used.
>   
>   See https://crates.io/crates/log for detailed info
> 
>
### Breaking changes
**PgSettings** ***executables_dir*** attribute has been removed (*described above*).

### Thanks
❤️ - Big thanks to **nicoulaj** for his contribution

# v0.4.3
___
- migrator fix

# v0.4.2
___
- updated documentation

# v0.4.1
___
- updated documentation

# v0.4.0
___
### Fix
 - changed file path vars from String to PathBuf
 - password authentication

### Feature
> - added authentication methods to **PgSettings**
>   
>   Setting the **auth_method** property of **PgSettings**
>   to one of the following values will determine the authentication
>   method:
> 
>   - **PgAuthMethod::Plain**
>       
>       Plain-Text password
>   - **PgAuthMethod::Md5**
>       
>       Md5 password hash
> 
>   - **PgAuthMethod::ScramSha256**
> 
>       Sha256 password hash
>
> 

### Breaking changes
**PgSettings** has a new property called **auth_method** (*described above*).

This property has to be set.

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

