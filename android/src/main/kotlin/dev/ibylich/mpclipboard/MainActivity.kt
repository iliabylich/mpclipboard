package dev.ibylich.mpclipboard

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.unit.dp

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val config = Store.readConfig(this)
        setContent {
            var host by rememberSaveable { mutableStateOf(config.host) }
            var token by rememberSaveable { mutableStateOf(config.token) }
            var id by rememberSaveable { mutableStateOf(config.id) }

            MaterialTheme {
                Surface {
                    Column(
                        modifier = Modifier
                            .safeDrawingPadding()
                            .padding(16.dp),
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
                            horizontalArrangement = Arrangement.End,
                        ) {
                            Button(
                                onClick = {
                                    host = host.trim()
                                    token = token.trim()
                                    id = id.trim()
                                    Store.writeConfig(this@MainActivity, Store.Config(host, token, id))
                                },
                            ) {
                                Text("Save")
                            }
                        }

                        Text(
                            text = "Version: ${BuildConfig.VERSION_NAME}",
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                }
            }
        }
    }
}
