package dev.ibylich.mpclipboard

import android.accounts.Account
import android.accounts.AccountManager
import android.content.Context
import android.os.Bundle

object Store {
    data class Config(
        val host: String,
        val token: String,
        val id: String,
    )

    const val ACCOUNT_TYPE = "dev.ibylich.mpclipboard"
    private const val ACCOUNT_NAME = "MPClipboard"

    private const val HOST = "host"
    private const val ID = "id"

    fun readConfig(context: Context): Config {
        val accountManager = AccountManager.get(context)
        val account = accountManager.getAccountsByType(ACCOUNT_TYPE).firstOrNull()
            ?: return Config(host = "", token = "", id = "")
        return Config(
            host = accountManager.getUserData(account, HOST).orEmpty(),
            token = accountManager.getPassword(account).orEmpty(),
            id = accountManager.getUserData(account, ID).orEmpty(),
        )
    }

    fun writeConfig(context: Context, config: Config) {
        val accountManager = AccountManager.get(context)
        val account = accountManager.getAccountsByType(ACCOUNT_TYPE).firstOrNull()
        if (account == null) {
            val userData = Bundle().apply {
                putString(HOST, config.host)
                putString(ID, config.id)
            }
            val added = accountManager.addAccountExplicitly(
                Account(ACCOUNT_NAME, ACCOUNT_TYPE),
                config.token,
                userData,
            )
            check(added) { "failed to add $ACCOUNT_TYPE account" }
        } else {
            accountManager.setPassword(account, config.token)
            accountManager.setUserData(account, HOST, config.host)
            accountManager.setUserData(account, ID, config.id)
        }
    }
}
