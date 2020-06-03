package mqtt

allow {
    input.operation.connect

    remote := input.operation.connect.remote_addr
    net.cidr_contains("127.0.0.1/16", remote)

    # outputs time as [hour, minute, sec]
    clock := time.clock([time.now_ns(), "America/Los_Angeles"])
    clock[0] >= 9
    clock[0] < 17
}

allow {
    input.operation.publish
}

allow {
    input.operation.subscribe
}

allow {
    input.operation.receive
}
