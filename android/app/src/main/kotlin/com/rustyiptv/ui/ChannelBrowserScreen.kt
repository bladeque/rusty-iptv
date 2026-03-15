package com.rustyiptv.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.tv.material3.*
import com.rustyiptv.bridge.ChannelSummary
import com.rustyiptv.bridge.CoreViewModel
import com.rustyiptv.bridge.FilterOptions

@OptIn(ExperimentalTvMaterial3Api::class)
@Composable
fun ChannelBrowserScreen(
    viewModel: CoreViewModel,
    onChannelSelected: (ChannelSummary) -> Unit,
    onChannelLongPress: (ChannelSummary) -> Unit
) {
    val channelPage by viewModel.channels.collectAsState()
    val loading by viewModel.loading.collectAsState()

    var currentPage by remember { mutableStateOf(0u) }
    var showHidden by remember { mutableStateOf(false) }

    LaunchedEffect(currentPage, showHidden) {
        viewModel.loadChannels(
            currentPage,
            FilterOptions(null, null, showHidden)
        )
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Color(0xFF1A1A2E))
    ) {
        // Filter bar
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(8.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(
                text = "Channels",
                style = MaterialTheme.typography.titleLarge,
                color = Color.White,
                modifier = Modifier.weight(1f)
            )
            Button(
                onClick = { showHidden = !showHidden; currentPage = 0u }
            ) {
                Text(if (showHidden) "Hide Hidden" else "Show Hidden")
            }
        }

        // Channel grid
        if (loading) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("Loading...", color = Color.White)
            }
        } else {
            channelPage?.let { page ->
                Column(modifier = Modifier.weight(1f)) {
                    // Simple list layout (TvLazyVerticalGrid requires more setup)
                    val chunked = page.channels.chunked(4)
                    chunked.forEach { rowChannels ->
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 8.dp, vertical = 4.dp),
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            rowChannels.forEach { channel ->
                                Box(modifier = Modifier.weight(1f)) {
                                    ChannelCard(
                                        channel = channel,
                                        onClick = { onChannelSelected(channel) }
                                    )
                                }
                            }
                            // Fill remaining slots
                            repeat(4 - rowChannels.size) {
                                Spacer(modifier = Modifier.weight(1f))
                            }
                        }
                    }
                }

                // Pagination controls
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(8.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Button(
                        onClick = { if (currentPage > 0u) currentPage-- },
                        enabled = currentPage > 0u
                    ) { Text("Previous") }

                    Text(
                        text = "Page ${currentPage + 1u} — ${page.total} total",
                        color = Color.White,
                        style = MaterialTheme.typography.bodyMedium
                    )

                    Button(
                        onClick = { currentPage++ },
                        enabled = page.channels.size.toUInt() == page.pageSize
                    ) { Text("Next") }
                }
            } ?: Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("No channels loaded", color = Color.Gray)
            }
        }
    }
}
