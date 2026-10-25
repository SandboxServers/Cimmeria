-- LoginInternal.lua, written by the Cimmeria launcher for one launch.
-- It is rewritten when the game exits.

LoginMod = {}

function LoginMod.loadServerSystems()
    LoginMod.servers = {}
    LoginMod.servers["Cimmeria"] = "http://127.0.0.1:50123"
    CimmeriaLogin = {}
    CimmeriaLogin.version = 1
    CimmeriaLogin.mode = "ticket"
    CimmeriaLogin.server = "Cimmeria"
    CimmeriaLogin.account = "Player_1234"
    CimmeriaLogin.ticket = "AbCdEf0123456789AbCdEf0123456789"
    CimmeriaLogin.display = "Player"
    CimmeriaLogin.auto = false
    CimmeriaLogin.links = {}
    CimmeriaLogin.errors = {}
    CimmeriaLogin.errors[4] = "Your sign-in for this launch is no longer valid. Close the game and press Play in the launcher."
    CimmeriaLogin.errors[5] = "This account is suspended. Contact the server staff."
    if LoginMod.cimmeriaOverlayVersion == nil then
        Login_AccountEdit:setText(CimmeriaLogin.account)
        Login_PasswordEdit:setText(CimmeriaLogin.ticket)
    end
end
