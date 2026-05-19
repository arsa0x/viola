return {
    triggers = { "lua", "ping-lua" },
    exec = function(ctx)
        ctx:reply("pong from lua!\ntime" .. ctx:elapsed_ms() .. "ms")
    end
}
