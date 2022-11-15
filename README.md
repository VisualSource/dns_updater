# Google Dyanmic DNS Updater

A simple updater for updating DNS records from a list with the current networks external IP address.

## API Reference

#### config.json

The main config file for setting options that the updater will use when running.
If the file does not exits it will be created.

```json
{
    "domains": [
        {
            "usr": "USERNAME",
            "psd": "PASSWORLD",
            "domain": "www.example.com"
        }
    ],  
    "debug": false,
}
```

| Parameter | Type     | Description                |
| :-------- | :------- | :------------------------- |
| `domains` | `Array<Object>` | **Required**. The list of domains that are to be updated |
| `debug`   | `boolean` | **Required**. Debugging output |
| `debug_ip`| `string` | When debug is true, this is the address used rather then using the networks external address |

#### dns_errored.json

A list of domains that have received error responses. excluding error response 911.
This is done so that the risk of getting your client blocked by google is avoided.
To reenable a given domain simpley remove the domain name from the array.

```json
    ["www.example.com"]
```

| Parameter | Type     | Description                       |
| :-------- | :------- | :-------------------------------- |
| `Array`      | `Array<String>` | **Required**. Id of item to fetch |