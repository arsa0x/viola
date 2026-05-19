return {
    triggers = { "lua", "ping-lua" },
    description = "Check bot latency and response time",
    exec = function(ctx)
        ctx:reply("pong from lua!\nreply took: " .. ctx:elapsed_ms_f64() .. "ms")
    end
}
