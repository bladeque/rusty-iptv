package com.rustyiptv.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.tv.material3.*
import com.rustyiptv.bridge.ChannelSummary
import com.rustyiptv.bridge.CoreViewModel

@OptIn(ExperimentalTvMaterial3Api::class)
@Composable
fun HomeScreen(viewModel: CoreViewModel, onChannelSelected: (ChannelSummary) -> Unit) {
    val channelPage by viewModel.channels.collectAsState()
    val loading by viewModel.loading.collectAsState()

    LaunchedEffect(Unit) { viewModel.loadChannels() }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Color(0xFF1A1A2E))
            .padding(16.dp)
    ) {
        Text(
            text = "rusty-iptv",
            style = MaterialTheme.typography.displaySmall,
            color = Color.White
        )
        Spacer(modifier = Modifier.height(16.dp))

        if (loading) {
            Text("Loading channels...", color = Color.White)
        } else {
            channelPage?.let { page ->
                if (page.channels.isEmpty()) {
                    Text("No channels. Add a provider to get started.", color = Color.Gray)
                } else {
                    // Group channels by group_title for category rows
                    val groups = page.channels.groupBy { it.groupTitle ?: "Other" }
                    groups.forEach { (groupName, channels) ->
                        Text(
                            text = groupName,
                            style = MaterialTheme.typography.titleMedium,
                            color = Color.White
                        )
                        Spacer(Modifier.height(8.dp))
                        Row(
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            channels.take(8).forEach { channel ->
                                ChannelCard(
                                    channel = channel,
                                    onClick = { onChannelSelected(channel) }
                                )
                            }
                        }
                        Spacer(Modifier.height(16.dp))
                    }
                }
            } ?: Text("Loading...", color = Color.Gray)
        }
    }
}

@OptIn(ExperimentalTvMaterial3Api::class)
@Composable
fun ChannelCard(channel: ChannelSummary, onClick: () -> Unit) {
    Card(
        onClick = onClick,
        modifier = Modifier
            .width(160.dp)
            .height(100.dp)
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(8.dp)
        ) {
            Column {
                Text(
                    text = channel.name,
                    style = MaterialTheme.typography.bodyMedium,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis
                )
                channel.groupTitle?.let {
                    Text(
                        text = it,
                        style = MaterialTheme.typography.labelSmall,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis
                    )
                }
                if (channel.isFavorite) {
                    Text("★", style = MaterialTheme.typography.labelSmall)
                }
            }
        }
    }
}
