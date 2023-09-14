# Smoothy-Client

![ ](https://img.shields.io/github/repo-size/y0Phoenix/Smoothy-Client)
![ ](https://img.shields.io/github/issues/y0Phoenix/Smoothy-Client)
![ ](https://img.shields.io/github/stars/y0Phoenix/Smoothy-Client)
![ ](https://img.shields.io/github/license/y0Phoenix/Smoothy-Client)

Client Application Taking Care Of Running, Logging and Restarting Smoothy In The Event Of A Crash

![alt text](https://github.com/y0Phoenix/Smoothy/blob/development/pictures/Smoothy%20Logo.png?raw=true)

## Usage

The only thing custom that would be needed to change is the `client_config.json` file you will need to create in a directory called `config` so the config files path should be `config/client_config.json`

Here are the parameters of this file

```json
{
    "restart_time": "00:00",
    "server_folder": "server",
    "global_data_file": "server/config/global.json",
    "max_file_size": 1000000000,
    "max_crash_count": 2,
    "crash_count_timer_len_in_millis": 300000
}
```

`restart_time` the time at which Smoothy would be best suited for a scheduled restart, ie. When the user count is particularly low.
`server_folder` the root directory of Smoothy.
`global_data_file` the path to the `global.json` file which contains all the server connection info for all the servers that Smoothy is connected to.
`max_file_size` the absolute maximum size a log file should be in Bytes. So the example is 1GB or 1 billion bytes.
`max_crash_count` the maximum amount of time Smoothy can crash within the specified timeframe of the next param without attemping to fix the issue.
`crash_count_timer_in_len_millis` as stated above, this is the time in milliseconds after a crash occurs to determine whether or not to attempt to fix More about that below.
the crash, ie. Clear the `global.json` file so Smoothy will no longer connect to the previous connected servers with and attempt to play their songs. Most issues Smoothy has
braches off from the fact that it's attempting to connect or play a song to a server.

## Open-Source

This project is Open-Source, and as such you can contribute as you would any other project via a pull-request.
