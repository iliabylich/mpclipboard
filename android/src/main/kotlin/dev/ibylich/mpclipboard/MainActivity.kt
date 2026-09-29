package dev.ibylich.mpclipboard

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.ui.Modifier
import dev.ibylich.mpclipboard.ui.SettingsScreen

class MainActivity : ComponentActivity() {
    private val requestLocalNetworkPermission =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) {}

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (savedInstanceState == null) {
            requestLocalNetworkPermissionIfNeeded()
        }
        setContent {
            MaterialTheme {
                Surface {
                    SettingsScreen(modifier = Modifier.safeDrawingPadding())
                }
            }
        }
    }

    private fun requestLocalNetworkPermissionIfNeeded() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.CINNAMON_BUN) {
            return
        }
        if (checkSelfPermission(Manifest.permission.ACCESS_LOCAL_NETWORK) == PackageManager.PERMISSION_GRANTED) {
            return
        }
        requestLocalNetworkPermission.launch(Manifest.permission.ACCESS_LOCAL_NETWORK)
    }
}
