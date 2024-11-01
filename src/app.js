maxEntriesToLoad = 100;

angular.module('app', ['ngMaterial'])
    .config(function($locationProvider) {
        $locationProvider.html5Mode(true);
    });


angular.module('app').component('wordnet', {
    templateUrl: 'static/wordnet.html',
    controller: function($scope) {
        $scope.synsets = [];
        $scope.focus = "";
        $scope.display = {
            show_wn31: true,
            display: false,
            langs: false
        };
    }
});


angular.module('app').controller('SearchController',
        function($scope, $http, $location) {
    var self = this;
    self.index = 'lemma';
    self.results = [];
    self.query_cleared = false;
    self.query = '';
    var m = $location.path().match("/(.*)/(.*)");
    self.selectedItemChange = function(item) {
        if(item) {
            $location.path("/"+ self.index + "/"+item.item);
            $scope.$parent.focus = item.item;
            self.results = [];
            self.query_cleared = true;
            $http.get("/json/"+ self.index + "/"+item.item).then(
                function(result) {
                    $scope.$parent.synsets = result.data;
                    $scope.$parent.link = self.index + "/" + item.item;
                }, function(response) {
                    console.log(response.data);
                });
        } else {
            $scope.$parent.focus = "";
            $scope.$parent.synsets = [];
            $scope.$parent.link = "";
        }
    };

    if(m) {
        self.index = m[1];
        if(m[2].endsWith(".html")) {
            self.selectedItem = { item: m[2].substring(0, m[2].length - 5), display: m[2] };
        } else {
            self.selectedItem = { item: m[2], display: m[2] };
        }
        self.selectedItemChange(self.selectedItem);
    }
    self.querySearch = function ($timeout) {
        if(self.query === "") {
            self.results = [];
        } else {
            var searched = self.query;
       $http.get("/autocomplete/"+ self.index + "/"+ self.query).then(
            function(result) {
                // See https://github.com/angular/material/issues/6668
                //var outerContainer = document.querySelector('.md-autocomplete-suggestions-container');
                //var innerContainer = document.querySelector('.md-virtual-repeat-sizer');
                //outerContainer.style.height = innerContainer.style.height;
                if(self.query === searched) {
                    self.results = result.data;
                    self.query_cleared = false;
                }
            }, function(response) {
                console.log(response.data);
            });
        }
    };
    self.keyPress = function(keyCode) {
        if(keyCode == 13) {
            if(self.results.length > 0 && self.results[0].item === self.query) {
                self.selectedItemChange(self.results[0]);
            }
        }
    }
});

angular.module('app').component('synset', {
        templateUrl: '/static/synset.html',
        bindings: {
            synset: '=',
            display: '<',
            focus: '='
        },
        controller: function($http) {
            var ctrl = this;
            ctrl.targetsynsets = [];
            $http.get("/json_rel/" + this.synset.id).then(
                    function(response) {
                        ctrl.targetsynsets = response.data.slice(0,maxEntriesToLoad);
                        ctrl.targetsynsets = ctrl.targetsynsets.filter((value, index, self) =>
                            index === self.findIndex((t) => (
                                t.id === value.id
                            ))
                        );
                        ctrl.targetsynsetsextra = response.data.slice(maxEntriesToLoad,response.data.length);
                    }, function(response) { /*alert(response);*/ }
            );
            ctrl.hasSubcats = function() {
                for(i = 0; i < this.synset.lemmas.length; i++) {
                    if(this.synset.lemmas[i].subcats.length > 0) {
                        return true;
                    }
                }
                return false;
            };
            ctrl.underlineSubcat = function(subcat, lemma) {
                if(lemma.includes(' ') && subcat.includes('----s')) {
                    let words = lemma.split(' ');
                    let word1 = words.shift();
                    let subcat2 = subcat.replace('----s', "----s " + words.join(" "));
                    return subcat2.replace('----', '<span class="underline">' + word1 + '</span>');
                } else {
                     return subcat.replace('----', '<span class="underline">' + lemma + '</span>');
                }
            };
            ctrl.extendtargetsynsets = function() {
                ctrl.targetsynsets = ctrl.targetsynsets.concat(
                        ctrl.targetsynsetsextra.slice(0,maxEntriesToLoad));
                ctrl.targetsynsetsextra = ctrl.targetsynsetsextra.slice(maxEntriesToLoad,ctrl.targetsynsetsextra.length);
            };
            ctrl.getSubcats = function(synset) {
                let subcats = {};
                for(i = 0; i < synset.lemmas.length; i++) {
                    let lemma = synset.lemmas[i].lemma;
                    for(j = 0; j < synset.lemmas[i].subcats.length; j++) {
                        let subcat = synset.lemmas[i].subcats[j];
                        if (subcats[subcat] === undefined) {
                            subcats[subcat] = [];
                        }
                        subcats[subcat].push(lemma);
                    }
                }
                // Convert dict to list
                subcats = Object.keys(subcats).map(function(key) {
                    return [key, subcats[key]];
                });
                return subcats;
            };
            ctrl.thirdPersonForm = function(word) {
                if (word.endsWith('s')) {
                    return word + "es";
                } else if (word.endsWith('ay') || word.endsWith('ey') || word.endsWith('iy') || word.endsWith('oy') || word.endsWith('uy')) {
                    return word + "s";
                } else if (word.endsWith('y')) {
                    return word.slice(0, -1) + "ies";
                } else if (word.endsWith('e')) {
                    return word + "s";
                } else if (word.endsWith('o')) {
                    return word + "es";
                } else if (word.endsWith('ch')) {
                    return word + "es";
                } else if (word.endsWith('sh')) {
                    return word + "es";
                } else if (word.endsWith('x')) {
                    return word + "es";
                } else {
                    return word + "s";
                }
            };
            ctrl.gerundForm = function(word) {
                if (word.endsWith('e')) {
                    return word.slice(0, -1) + "ing";
                } else if (word.endsWith('ie')) {
                    return word.slice(0, -2) + "ying";
                } else {
                    return word + "ing";
                }
            };
            ctrl.replaceSubcat = function(subcats) {
                let subcat = subcats[0];
                let mapped_lemmas = [];
                if(subcat.includes('----s')) {
                    for(i = 0; i < subcats[1].length; i++) {
                        if (subcats[1][i].includes(' ')) {
                            mapped_lemmas.push(
                                ctrl.thirdPersonForm(subcats[1][i].split(' ')[0]) + " " + subcats[1][i].split(' ').slice(1).join(' '));
                        } else {
                            mapped_lemmas.push(ctrl.thirdPersonForm(subcats[1][i]));
                        }
                    }
                    return subcat.replace('----s', mapped_lemmas.join('/'));
                } else if(subcat.includes('----ing')) {
                    for(i = 0; i < subcats[1].length; i++) {
                        if (subcats[1][i].includes(' ')) {
                            mapped_lemmas.push(
                                ctrl.gerundForm(subcats[1][i].split(' ')[0]) + " " + subcats[1][i].split(' ').slice(1).join(' '));
                        } else {
                            mapped_lemmas.push(ctrl.gerundForm(subcats[1][i]));
                        }
                    }
                    return subcat.replace('----ing', mapped_lemmas.join('/'));
                } else {
                    for(i = 0; i < subcats[1].length; i++) {
                        mapped_lemmas.push(subcats[1][i]);
                    }
                    return subcat.replace('----', mapped_lemmas.join('/'));
                }
            };
        }
    });

angular.module('app').component('synset2', {
    templateUrl: '/static/synset.html',
    bindings: {
        synset: '<',
        display: '<',
        focus: '='
    },
    controller: function($http) {
        var ctrl = this;
        ctrl.targetsynsets = [];
        $http.get("/json_rel/" + this.synset.id).then(
                function(response) {
                    ctrl.targetsynsets = response.data.slice(0,maxEntriesToLoad);
                    ctrl.targetsynsets = ctrl.targetsynsets.filter((value, index, self) =>
                        index === self.findIndex((t) => (
                            t.id === value.id
                        ))
                    );
                    ctrl.targetsynsetsextra = response.data.slice(maxEntriesToLoad,response.data.length);
                }, function(response) { /*alert(response);*/ }
        );
        ctrl.hasSubcats = function() {
            for(i = 0; i < this.synset.lemmas.length; i++) {
                if(this.synset.lemmas[i].subcats.length > 0) {
                    return true;
                }
            }
            return false;
        };
        ctrl.underlineSubcat = function(subcat, lemma) {
            if(lemma.includes(' ') && subcat.includes('----s')) {
                let words = lemma.split(' ');
                let word1 = words.shift();
                let subcat2 = subcat.replace('----s', "----s " + words.join(" "));
                return subcat2.replace('----', '<span class="underline">' + word1 + '</span>');
            } else {
                return subcat.replace('----', '<span class="underline">' + lemma + '</span>');
            }
        };
        ctrl.extendtargetsynsets = function() {
            ctrl.targetsynsets = ctrl.targetsynsets.concat(
                    ctrl.targetsynsetsextra.slice(0,maxEntriesToLoad));
            ctrl.targetsynsetsextra = ctrl.targetsynsetsextra.slice(maxEntriesToLoad,ctrl.targetsynsetsextra.length);
        };
        ctrl.replaceSubcat = function(subcat, lemma) {
            if(lemma.includes(' ') && subcat.includes('----s')) {
                let words = lemma.split(' ');
                let word1 = words.shift();
                let subcat2 = subcat.replace('----s', "----s " + words.join(" "));
                return subcat2.replace('----', word1);
            } else {
                return subcat.replace('----', lemma);
            }
        };
 }
    //,
//    controller: function($http) {
//        var ctrl = this;
//        ctrl.synset = null;
//        for(i = 0; i < target_synsets.length; i++) {
//            if(ctrl.target === target_synsets[j].id) {
//                ctrl.synset = target_synsets[j];
//            }
//        }
//        if(ctrl.synset == null) {
//            $http.get("/json/id/" + ctrl.target).then(
//                function(response) {
//                    ctrl.synset = response.data[0];
//                },
//                function(response) {
//                    console.log(response.data);
//                });
//        }
//    }
});

angular.module('app').component('relation', {
        templateUrl: '/static/relation.html',
        bindings: {
            fullname: '@',
            relation: '@',
            relations: '=',
            display: '<',
            targetsynsets: '<'
        },
        controller: function($http) {
            var ctrl = this;
            this.show = false;
        }
//        controller: function($scope,$http) {
//            console.log("Loading synsets");
//            for(r = 0; r < this.relations.length; r++) {
//                var relation = this.relations[r];
//                console.log(relation);
//                $http.get("/json/id/" + relation.target).then(
//                    function(response) {
//                        console.log(response.data);
//                        relation.synset = response.data[0];
//                        console.log(relation);
//                    },
//                    function(response) {
//                        console.log(response.data);
//                    });
//            }
//        }

    });


angular.module('app').filter('isAdjective', function() {
    return function(items) {
        var filtered = [];
        for(var i = 0; i < items.length; i++) {
            var item = items[i];
            if(item.pos === 'a' || item.pos === 's') {
                filtered.push(item);
            }
        }
        return filtered;
    }
});

// From https://github.com/EricWVGG/AngularSlideables
angular.module('app')
.directive('slideable', function () {
    return {
        restrict:'C',
        compile: function (element, attr) {
            // wrap tag
            var contents = element.html();
            element.html('<div class="slideable_content" style="margin:0 !important; padding:0 !important" >' + contents + '</div>');

            return function postLink(scope, element, attrs) {
                // default properties
                attrs.duration = (!attrs.duration) ? '.3s' : attrs.duration;
                attrs.easing = (!attrs.easing) ? 'ease-in-out' : attrs.easing;
                element.css({
                    'overflow': 'hidden',
                    'height': '0px',
                    'transitionProperty': 'height',
                    'transitionDuration': attrs.duration,
                    'transitionTimingFunction': attrs.easing
                });
            };
        }
    };
})
.directive('slideToggle', function() {
    return {
        restrict: 'A',
        link: function(scope, element, attrs) {
            var target = document.querySelector(attrs.slideToggle);
            attrs.expanded = false;
            element.bind('click', function() {
                var content = target.querySelector('.slideable_content');
                if(!attrs.expanded) {
                    content.style.border = '1px solid rgba(0,0,0,0)';
                    var y = content.clientHeight;
                    content.style.border = 0;
                    target.style.height = y + 'px';
                } else {
                    target.style.height = '0px';
                }
                attrs.expanded = !attrs.expanded;
            });
        }
    }
});

angular.module('app').run(function($rootScope, $location){
   //Bind the `$locationChangeSuccess` event on the rootScope, so that we dont need to 
   //bind in induvidual controllers.

   $rootScope.$on('$locationChangeSuccess', function() {
        $rootScope.actualLocation = $location.path();
    });        

   $rootScope.$watch(function () {return $location.path()}, function (newLocation, oldLocation) {
        if($rootScope.actualLocation === newLocation) {
            location.reload();
        }
    });
});
