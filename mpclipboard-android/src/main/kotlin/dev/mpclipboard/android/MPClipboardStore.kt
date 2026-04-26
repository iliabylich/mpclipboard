package dev.mpclipboard.android

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

private val Context.mpClipboardDataStore: DataStore<Preferences> by preferencesDataStore(
    name = "mpclipboard",
)

class MPClipboardStore private constructor(
    private val dataStore: DataStore<Preferences>,
) {
    val config: Flow<MPClipboardConfig> = dataStore.data.map { preferences ->
        MPClipboardConfig(
            host = preferences[HOST].orEmpty(),
            token = preferences[TOKEN].orEmpty(),
            name = preferences[NAME].orEmpty(),
        )
    }

    val connectivity: Flow<Connectivity> = dataStore.data.map { preferences ->
        preferences[CONNECTIVITY]?.let(Connectivity::valueOfOrNull) ?: Connectivity.Disconnected
    }

    suspend fun saveConfig(config: MPClipboardConfig) {
        dataStore.edit { preferences ->
            preferences[HOST] = config.host
            preferences[TOKEN] = config.token
            preferences[NAME] = config.name
        }
    }

    suspend fun saveConnectivity(connectivity: Connectivity) {
        dataStore.edit { preferences ->
            preferences[CONNECTIVITY] = connectivity.name
        }
    }

    companion object {
        private val HOST = stringPreferencesKey("host")
        private val TOKEN = stringPreferencesKey("token")
        private val NAME = stringPreferencesKey("name")
        private val CONNECTIVITY = stringPreferencesKey("connectivity")

        fun from(context: Context): MPClipboardStore {
            return MPClipboardStore(context.applicationContext.mpClipboardDataStore)
        }
    }
}
