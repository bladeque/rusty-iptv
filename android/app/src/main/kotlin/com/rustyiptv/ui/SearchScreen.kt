package com.rustyiptv.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.tv.material3.*
import com.rustyiptv.bridge.ChannelSummary
import com.rustyiptv.bridge.CoreViewModel
import kotlinx.coroutines.FlowPreview
import kotlinx.coroutines.flow.*

@OptIn(ExperimentalTvMaterial3Api::class, FlowPreview::class)
@Composable
fun SearchScreen(
    viewModel: CoreViewModel,
    onChannelSelected: (ChannelSummary) -> Unit
) {
    var query by remember { mutableStateOf("") }
    val searchResults by viewModel.searchResults.collectAsState()

    // Debounce search input
    val queryFlow = remember { MutableStateFlow("") }
    LaunchedEffect(Unit) {
        queryFlow
            .debounce(300)
            .distinctUntilChanged()
            .collect { q ->
                if (q.isNotBlank()) viewModel.search(q)
            }
    }

    LaunchedEffect(query) { queryFlow.value = query }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Color(0xFF1A1A2E))
            .padding(16.dp)
    ) {
        Text(
            text = "Search",
            style = MaterialTheme.typography.titleLarge,
            color = Color.White
        )
        Spacer(Modifier.height(12.dp))

        // Search input
        Surface(
            modifier = Modifier.fillMaxWidth(),
            shape = MaterialTheme.shapes.medium
        ) {
            BasicTextField(
                value = query,
                onValueChange = { query = it },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(12.dp),
                singleLine = true,
                decorationBox = { inner ->
                    if (query.isEmpty()) {
                        Text("Search channels...", color = Color.Gray)
                    }
                    inner()
                }
            )
        }

        Spacer(Modifier.height(12.dp))
        Text(
            text = "${searchResults.size} results",
            style = MaterialTheme.typography.labelMedium,
            color = Color.Gray
        )
        Spacer(Modifier.height(8.dp))

        // Results list
        Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            searchResults.forEach { channel ->
                ListItem(
                    selected = false,
                    onClick = { onChannelSelected(channel) },
                    headlineContent = {
                        Text(channel.name, color = Color.White)
                    },
                    supportingContent = channel.groupTitle?.let { group ->
                        { Text(group, color = Color.Gray) }
                    },
                    trailingContent = if (channel.isFavorite) {
                        { Text("★", color = Color.Yellow) }
                    } else null
                )
            }
        }
    }
}
