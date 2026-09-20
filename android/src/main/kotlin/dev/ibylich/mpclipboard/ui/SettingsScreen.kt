package dev.ibylich.mpclipboard.ui

import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.runtime.Composable
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import dev.ibylich.mpclipboard.Store
import kotlinx.coroutines.flow.first

@Composable
fun SettingsScreen(
    modifier: Modifier = Modifier,
    store: Store = rememberStore(),
) {
    val config = produceState<Store.Config?>(
        initialValue = null,
        key1 = store,
    ) {
        value = store.config.first()
    }.value

    if (config == null) {
        CircularProgressIndicator(modifier = modifier)
    } else {
        SettingsForm(
            initialHost = config.host,
            initialToken = config.token,
            initialId = config.id,
            store = store,
            modifier = modifier,
        )
    }
}

@Composable
private fun rememberStore(): Store {
    val context = LocalContext.current.applicationContext
    return remember(context) { Store.from(context) }
}
