package dev.ibylich.mpclipboard.ui

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
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp
import dev.ibylich.mpclipboard.Ffi.Connectivity
import dev.ibylich.mpclipboard.Store
import kotlinx.coroutines.launch

@Composable
fun SettingsForm(
    initialHost: String,
    initialToken: String,
    initialId: String,
    store: Store,
    modifier: Modifier = Modifier,
) {
    val connectivity by store.connectivity.collectAsState(initial = Connectivity.Disconnected)
    val scope = rememberCoroutineScope()
    var host by rememberSaveable { mutableStateOf(initialHost) }
    var token by rememberSaveable { mutableStateOf(initialToken) }
    var id by rememberSaveable { mutableStateOf(initialId) }

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
            value = id,
            onValueChange = { id = it },
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
                        host = host.trim()
                        token = token.trim()
                        id = id.trim()
                        store.writeConfig(host, token, id)
                    }
                },
            ) {
                Text("Save")
            }
        }
    }
}
