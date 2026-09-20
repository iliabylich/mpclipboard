package dev.ibylich.mpclipboard

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map

private val Context.mpClipboardDataStore: DataStore<Preferences> by preferencesDataStore(
    name = "mpclipboard",
)

class Store private constructor(
    private val dataStore: DataStore<Preferences>,
) {
    data class Config(
        val host: String,
        val token: String,
        val id: String,
    )

    val host: Flow<String> = dataStore.data
        .map { preferences -> preferences[HOST].orEmpty() }
        .distinctUntilChanged()

    val token: Flow<String> = dataStore.data
        .map { preferences -> preferences[TOKEN].orEmpty() }
        .distinctUntilChanged()

    val id: Flow<String> = dataStore.data
        .map { preferences -> preferences[ID].orEmpty() }
        .distinctUntilChanged()

    val config: Flow<Config> = dataStore.data
        .map { preferences ->
            Config(
                host = preferences[HOST].orEmpty(),
                token = preferences[TOKEN].orEmpty(),
                id = preferences[ID].orEmpty(),
            )
        }
        .distinctUntilChanged()

    val connectivity: Flow<Ffi.Connectivity> = dataStore.data
        .map { preferences ->
            preferences[CONNECTIVITY]
                ?.let(Ffi.Connectivity::valueOfOrNull)
                ?: Ffi.Connectivity.Disconnected
        }
        .distinctUntilChanged()

    suspend fun writeConfig(host: String, token: String, id: String) {
        dataStore.edit { preferences ->
            preferences[HOST] = host
            preferences[TOKEN] = token
            preferences[ID] = id
        }
    }

    suspend fun writeConnectivity(connectivity: Ffi.Connectivity) {
        dataStore.edit { preferences ->
            preferences[CONNECTIVITY] = connectivity.name
        }
    }

    companion object {
        private val HOST = stringPreferencesKey("host")
        private val TOKEN = stringPreferencesKey("token")
        private val ID = stringPreferencesKey("name")
        private val CONNECTIVITY = stringPreferencesKey("connectivity")

        fun from(context: Context): Store {
            return Store(context.applicationContext.mpClipboardDataStore)
        }
    }
}
