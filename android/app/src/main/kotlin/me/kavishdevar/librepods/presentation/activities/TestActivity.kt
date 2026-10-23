/*
    LibrePods - AirPods liberated from Apple’s ecosystem
    Copyright (C) 2025 LibrePods contributors

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

package me.kavishdevar.librepods.presentation.activities

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.serialization.cbor.Cbor
import kotlinx.serialization.decodeFromByteArray
import kotlinx.serialization.encodeToByteArray
import me.kavishdevar.librepods.data.app.IslandSettings
import me.kavishdevar.librepods.data.app.ProximityCardSettings
import java.io.File

class TestActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        val cbor = Cbor { ignoreUnknownKeys = true }
        val defaultSettings = ProximityCardSettings()
        val bytes = cbor.encodeToByteArray(defaultSettings)

        val hexString = "X'" + bytes.joinToString("") { "%02X".format(it) } + "'"

        val file = File(filesDir, "proximity_default.txt")
        file.writeText(hexString)

        setContent {
            Scaffold(modifier = Modifier.fillMaxSize()) { innerPadding ->
                Column (
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(innerPadding)
                        .padding(16.dp),
                    verticalArrangement = Arrangement.SpaceAround
                ) {
                    Text(text = "Exported Default Hex:\n\n$hexString")

                    val decoded = cbor.decodeFromByteArray<IslandSettings>(
                        byteArrayOf(0xBF.toByte(), 0xFF.toByte())
                    )

                    Text(text = "Decoded Default:\n\n$decoded")
                }
            }
        }
    }
}
