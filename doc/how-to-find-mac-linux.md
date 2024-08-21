## How to find your mac address (Linux)

On Linux, you can use the `ip -c addr` command to find the mac address:

```
$ ip -c addr
...
3: enp36s0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc mq state UP group default qlen 1000
    link/ether 00:11:22:33:44:55 brd ff:ff:ff:ff:ff:ff
    inet 10.12.0.20/24 brd 10.12.0.255 scope global enp36s0
       valid_lft forever preferred_lft forever
...
```

Here we see an interface called `enp36s0` that has the mac address `00:11:22:33:44:55`.

You will see multiple interfaces - use the one with your IP address listed.
