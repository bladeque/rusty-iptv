package com.rustyiptv

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.tv.material3.ExperimentalTvMaterial3Api
import androidx.tv.material3.MaterialTheme
import com.rustyiptv.bridge.CoreViewModel
import com.rustyiptv.ui.AppNavigation

class MainActivity : ComponentActivity() {
    private val viewModel: CoreViewModel by viewModels()

    @OptIn(ExperimentalTvMaterial3Api::class)
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                AppNavigation(viewModel = viewModel)
            }
        }
    }
}
