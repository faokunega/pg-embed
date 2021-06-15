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

