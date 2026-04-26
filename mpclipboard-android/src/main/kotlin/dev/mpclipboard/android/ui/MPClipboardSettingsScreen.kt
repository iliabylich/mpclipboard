package dev.mpclipboard.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import dev.mpclipboard.android.Connectivity
import dev.mpclipboard.android.MPClipboardConfig
import dev.mpclipboard.android.MPClipboardStore
import kotlinx.coroutines.launch

@Composable
fun rememberMPClipboardStore(): MPClipboardStore {
    val context = LocalContext.current.applicationContext
    return remember(context) {
        MPClipboardStore.from(context)
    }
}

@Composable
fun MPClipboardSettingsScreen(
    modifier: Modifier = Modifier,
    store: MPClipboardStore = rememberMPClipboardStore(),
) {
    val savedConfig by store.config.collectAsState(initial = MPClipboardConfig())
    val connectivity by store.connectivity.collectAsState(initial = Connectivity.Disconnected)
    val scope = rememberCoroutineScope()
    var didLoadSavedConfig by rememberSaveable { mutableStateOf(false) }
    var host by rememberSaveable { mutableStateOf("") }
    var token by rememberSaveable { mutableStateOf("") }
    var name by rememberSaveable { mutableStateOf("") }

    LaunchedEffect(savedConfig) {
        if (!didLoadSavedConfig) {
            host = savedConfig.host
            token = savedConfig.token
            name = savedConfig.name
            didLoadSavedConfig = true
        }
    }

    Column(
        modifier = modifier.padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(
            text = "MPClipboard",
            style = MaterialTheme.typography.titleLarge,
        )

        OutlinedTextField(
            value = host,
            onValueChange = { host = it },
            modifier = Modifier.fillMaxWidth(),
            label = { Text("Host") },
            singleLine = true,
        )

        OutlinedTextField(
            value = token,
            onValueChange = { token = it },
            modifier = Modifier.fillMaxWidth(),
            label = { Text("Token") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
        )

        OutlinedTextField(
            value = name,
            onValueChange = { name = it },
            modifier = Modifier.fillMaxWidth(),
            label = { Text("Name") },
            singleLine = true,
        )

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = connectivity.name,
                style = MaterialTheme.typography.bodyMedium,
            )

            Button(
                onClick = {
                    scope.launch {
                        store.saveConfig(
                            MPClipboardConfig(
                                host = host.trim(),
                                token = token.trim(),
                                name = name.trim(),
                            ),
                        )
                    }
                },
            ) {
                Text("Save")
            }
        }
    }
}
