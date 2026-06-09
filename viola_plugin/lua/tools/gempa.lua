return {
    name = "BMKB",
    triggers = { "gempa", },
    description = "testing http request",
    exec = function(ctx)
        local response = http:get("https://data.bmkg.go.id/DataMKG/TEWS/autogempa.json")

        if response:status() == 200 then
            ctx:reply("response: " .. response:status())
            ctx:reply_success()
        else
            ctx:reply_failed()
        end
    end
}
