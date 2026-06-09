return {
    name = "ping",
    triggers = { "lua", "ping-lua" },
    description = "Check bot latency and response time",
    exec = function(ctx)
        ctx:reply("pong from lua!\nprocessing: " .. ctx:processing_ms() .. "ms")
    end
}
