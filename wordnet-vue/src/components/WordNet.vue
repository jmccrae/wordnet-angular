<script>
  export default {
    data() {
      return {
        index: 'lemma',
        query: '',
        results: []
      }
    },
    methods: {
      querySearch() {
        console.log('querySearch')
      },
      keyPress(keyCode) {
        console.log('keyPress', keyCode)
      }
    }
  }
</script>

<template>
  <div class="row">
    <div layout="column">
      <table width="100%">
        <tr>
          <td width="100px;" style="vertical-align:top;">
              <button class="md-button md-icon-button" @click="$mdMenu.open()">{{index}}</button>
              <ul class="menu" style="display:none;">
                <li class="md-menu-item" @click="index = 'lemma'">Lemma</li>
                <li class="md-menu-item" @click="index = 'id'">Identifier</li>
                <li class="md-menu-item" @click="index = 'ili'">Interlingual Identifier</li>
                <li class="md-menu-item" @click="index = 'sense_key'">Sense Key</li>
                <li class="md-menu-item" @click="index = 'pwn30'">WordNet 3.0</li>
                <li class="md-menu-item" @click="index = 'pwn21'">WordNet 2.1</li>
                <li class="md-menu-item" @click="index = 'pwn20'">WordNet 2.0</li>
                <li class="md-menu-item" @click="index = 'pwn171'">WordNet 1.7.1</li>
                <li class="md-menu-item" @click="index = 'pwn17'">WordNet 1.7</li>
                <li class="md-menu-item" @click="index = 'pwn16'">WordNet 1.6</li>
                <li class="md-menu-item" @click="index = 'pwn15'">WordNet 1.5</li>
              </ul>
          </td>
          <td>
            <input type="text" class="search-control" placeholder="Search"
                                                      @change="querySearch()"
                                                      @keyup="keyPress($event.keyCode)">
            <ul class="list-group search-group" v-if="results && results.length > 0">
              <li v-for="result in ctrl.results" class="list-group-item" @click="selectedItemChange(result)">
                <a>{{ result.display }}</a>
              </li>
            </ul>
            <ul class="list-group search-group" v-if="results && results.length == 0 && query.length > 0">
              <li class="list-group-item"><i>No results</i></li>
            </ul>
          </td>
        </tr>
      </table>
    </div>
    <div>
      <md-button slide-toggle="#display" class="pull-right option_button" @click="display.display = !display.display"
                                                                          ng-class="{option_button_selected: display.display}">Options  &#x25bc;</md-button>
      <md-button slide-toggle="#langs" class="pull-right option_button" @click="display.langs = !display.langs"
                                                                        ng-class="{option_button_selected: display.langs}">Translations   &#x25bc;</md-button>
    </div>
    <div id="display" class="slideable option_panel">
      <div class="option_panel_internal">
        <table>
          <tr>
            <td><md-checkbox ng-model="display.ids">Show Synset Identifier</md-checkbox></td>
            <td><md-checkbox ng-model="display.sensekeys">Show Sense Keys</md-checkbox></td>
            <td><md-checkbox ng-model="display.subcats">Show Subcategorization Frames</md-checkbox></td>
            <td><md-checkbox ng-model="display.topics">Show Topics</md-checkbox></td>
            <td><md-checkbox ng-model="display.wn30">Show WordNet 3.0 Identifer</md-checkbox></td>
            <td><md-checkbox ng-model="display.wn_old">Show Previous WordNet Identifiers</md-checkbox></td>
          </tr>
        </table>
      </div>
    </div>
    <div id="langs" class="slideable option_panel">
      <div class="option_panel_internal">
        <table>
          <tr>
            <td><md-checkbox ng-model="display.lang_als"><img src="../assets/flags/als.gif"/> Albanian</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_arb"><img src="../assets/flags/arb.gif"/> Arabic</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_ind"><img src="../assets/flags/ind.gif"/> Bahasa Indonesia</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_zsm"><img src="../assets/flags/zsm.gif"/> Bahasa Malay</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_eus"><img src="../assets/flags/eus.gif"/> Basque</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_bul"><img src="../assets/flags/bul.gif"/> Bulgarian</md-checkbox></td>
          </tr><tr>
            <td><md-checkbox ng-model="display.lang_cat"><img src="../assets/flags/cat.gif"/> Catalan</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_cmn"><img src="../assets/flags/cmn.gif"/> Chinese (simplified)</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_qcn"><img src="../assets/flags/qcn.gif"/> Chinese (traditional)</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_hrv"><img src="../assets/flags/hrv.gif"/> Croatian</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_nld"><img src="../assets/flags/nld.gif"/> Dutch</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_fin"><img src="../assets/flags/fin.gif"/> Finnish</md-checkbox></td>
          </tr><tr>
            <td><md-checkbox ng-model="display.lang_fra"><img src="../assets/flags/fra.gif"/> French</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_glg"><img src="../assets/flags/glg.gif"/> Galician</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_ell"><img src="../assets/flags/ell.gif"/> Greek</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_heb"><img src="../assets/flags/heb.gif"/> Hebrew</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_isl"><img src="../assets/flags/isl.gif"/> Icelandic</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_ita"><img src="../assets/flags/ita.gif"/> Italian</md-checkbox></td>
          </tr><tr>
            <td><md-checkbox ng-model="display.lang_jpn"><img src="../assets/flags/jpn.gif"/> Japanese</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_lit"><img src="../assets/flags/lit.gif"/> Lithuanian</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_nno"><img src="../assets/flags/nno.gif"/> Norwegian (Nynorsk)</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_nob"><img src="../assets/flags/nob.gif"/> Norwegian (Bokm&aring;l)</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_fas"><img src="../assets/flags/fas.gif"/> Persian</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_pol"><img src="../assets/flags/pol.gif"/> Polish</md-checkbox></td>
          </tr><tr>
            <td><md-checkbox ng-model="display.lang_por"><img src="../assets/flags/por.gif"/> Portuguese</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_ron"><img src="../assets/flags/ron.gif"/> Romanian</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_slk"><img src="../assets/flags/slk.gif"/> Slovakian</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_slv"><img src="../assets/flags/slv.gif"/> Slovene</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_spa"><img src="../assets/flags/spa.gif"/> Spanish</md-checkbox></td>
            <td><md-checkbox ng-model="display.lang_swe"><img src="../assets/flags/swe.gif"/> Swedish</md-checkbox></td>
          </tr><tr>
            <td><md-checkbox ng-model="display.lang_tha"><img src="../assets/flags/tha.gif"/> Thai</md-checkbox></td>
          </tr>
        </table>
      </div>
    </div>
    <span class="pos_grp">
      <h3 class="pos_label">Nouns</h3>
      <div ng-repeat="synset in synsets | filter:{'pos':'n'}">
        <synset synset="synset" display="display" focus="focus"></synset>
      </div>
    </span>
    <span class="pos_grp">
      <h3 class="pos_label">Verbs</h3>
      <div ng-repeat="synset in synsets | filter:{'pos':'v'}">
        <synset synset="synset" display="display" focus="focus"></synset>
      </div>
    </span>
    <span class="pos_grp">
      <h3 class="pos_label">Adverbs</h3>
      <div ng-repeat="synset in synsets | filter:{'pos':'r'}">
        <synset synset="synset" display="display" focus="focus"></synset>
      </div>
    </span>
    <span class="pos_grp">
      <h3 class="pos_label">Adjectives</h3>
      <div ng-repeat="synset in synsets | isAdjective">
        <synset synset="synset" display="display" focus="focus"></synset>
      </div>
    </span>
    <div class="pull-right" ng-show="link">
      <b>Download As:</b>&nbsp;&nbsp;<a target="_self" ng-href="/json/{{link}}">JSON</a>&nbsp;&nbsp;
      <a target="_self" ng-href="/ttl/{{link}}">RDF</a>&nbsp;&nbsp;
      <a target="_self" ng-href="/xml/{{link}}">XML</a>
    </div>
  </div>
</template>


